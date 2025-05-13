use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    // marker::PhantomData,
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use twilight_model::{
    channel::{ChannelType, message::MessageFlags},
    guild::Permissions,
    id::{Id, marker::ChannelMarker},
};

use crate::{
    LavalinkAndGuildIdAware,
    command::model::{Ctx, CtxKind},
    component::config::access::CalculatorBuilder,
    core::{
        model::{BotState, DatabaseAware, OwnedBotStateAware, UserIdAware, UserPermissionsAware},
        traced,
    },
    error::{
        Cache, InVoiceWithSomeoneElse as InVoiceWithSomeoneElseError,
        InVoiceWithoutUser as InVoiceWithoutUserError, NotUsersTrack as NotUsersTrackError,
        UserNotAccessManager as UserNotAccessManagerError, UserNotAllowed as UserNotAllowedError,
        UserNotDj as UserNotDjError, UserNotStageManager as UserNotStageManagerError,
        command::check::{
            self, AlternateVoteResponse, PollResolvableError, SendSupersededWinNoticeError,
            UserOnlyInError,
        },
    },
    gateway::GuildIdAware,
    lavalink::{
        self,
        CorrectTrackInfo,
        // DelegateMethods,
        Event,
        // PlayerAware,
        QueueItem,
    },
};

use super::{
    model::{GuildCtx, RespondViaMessage},
    poll::{self, Resolution as PollResolution, Topic as PollTopic},
    require::{self, CurrentTrack, InVoice, PartialInVoice, someone_else_in},
};

pub const DJ_PERMISSIONS: Permissions = Permissions::MOVE_MEMBERS.union(Permissions::MUTE_MEMBERS);
pub const ACCESS_MANAGER_PERMISSIONS: Permissions =
    Permissions::MANAGE_ROLES.union(Permissions::MANAGE_CHANNELS);

pub const STAGE_MANAGER_PERMISSIONS: Permissions = Permissions::MANAGE_CHANNELS
    .union(Permissions::MUTE_MEMBERS)
    .union(Permissions::MOVE_MEMBERS);

pub fn does_user_have_permissions(
    permissions: Permissions,
    ctx: &impl UserPermissionsAware,
) -> bool {
    let author_permissions = ctx.user_permissions();
    author_permissions.contains(permissions)
        || author_permissions.contains(Permissions::ADMINISTRATOR)
}

#[inline]
pub fn is_user_dj(ctx: &impl UserPermissionsAware) -> bool {
    does_user_have_permissions(DJ_PERMISSIONS, ctx)
}

pub fn user_is_dj(ctx: &impl UserPermissionsAware) -> Result<(), UserNotDjError> {
    if !is_user_dj(ctx) {
        return Err(UserNotDjError);
    }
    Ok(())
}

pub fn user_is_access_manager(
    ctx: &impl UserPermissionsAware,
) -> Result<(), UserNotAccessManagerError> {
    if !does_user_have_permissions(ACCESS_MANAGER_PERMISSIONS, ctx) {
        return Err(UserNotAccessManagerError);
    }
    Ok(())
}

pub fn user_is_stage_manager(
    ctx: &impl UserPermissionsAware,
) -> Result<(), UserNotStageManagerError> {
    if !does_user_have_permissions(STAGE_MANAGER_PERMISSIONS, ctx) {
        return Err(UserNotStageManagerError);
    }
    Ok(())
}

pub async fn user_allowed_in(ctx: &Ctx<impl CtxKind>) -> Result<(), check::UserAllowedError> {
    let Some(weak) = require::guild_ref(ctx).ok() else {
        return Ok(());
    };

    if user_is_access_manager(&weak).is_ok() {
        return Ok(());
    }

    let channel = ctx.channel();
    let mut access_calculator_builder = CalculatorBuilder::new(weak.guild_id(), ctx.db().clone())
        .user(ctx.user_id())
        .roles(weak.member().roles.iter());
    match channel.kind {
        ChannelType::PublicThread
        | ChannelType::PrivateThread
        | ChannelType::AnnouncementThread => {
            let parent_id = channel
                .parent_id
                .expect("channel of thread types should have a parent");
            access_calculator_builder = access_calculator_builder
                .thread(channel.id)
                .text_channel(parent_id);
        }
        ChannelType::GuildVoice | ChannelType::GuildStageVoice => {
            let channel_id = channel.id;
            access_calculator_builder = access_calculator_builder
                .text_channel(channel_id)
                .voice_channel(channel_id);
        }
        _ => {
            access_calculator_builder = access_calculator_builder.text_channel(channel.id);
            if let Some(category_channel_id) = channel.parent_id {
                access_calculator_builder =
                    access_calculator_builder.category_channel(category_channel_id);
            }
        }
    }

    let user_allowed_to_use_commands = access_calculator_builder.build().await?.calculate();
    if !user_allowed_to_use_commands {
        return Err(UserNotAllowedError.into());
    }
    Ok(())
}

pub async fn user_allowed_to_use(
    channel_id: Id<ChannelMarker>,
    channel_parent_id: Option<Id<ChannelMarker>>,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(), check::UserAllowedError> {
    if user_is_access_manager(ctx).is_ok() {
        return Ok(());
    }

    let guild_id = ctx.guild_id();
    let mut access_calculator_builder =
        CalculatorBuilder::new(guild_id, ctx.db().clone()).voice_channel(channel_id);

    if let Some(parent_id) = channel_parent_id {
        access_calculator_builder = access_calculator_builder.category_channel(parent_id);
    }

    let allowed_to_use_channel = access_calculator_builder.build().await?.calculate();

    if !allowed_to_use_channel {
        return Err(UserNotAllowedError.into());
    }
    Ok(())
}

pub struct InVoiceWithUserResult<'a> {
    in_voice: InVoice<'a>,
    kind: InVoiceWithUserResultKind,
}

pub enum InVoiceWithUserResultKind {
    UserIsDj,
    ToBeDetermined,
}

pub fn noone_else_in(
    in_voice: PartialInVoice,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(), check::UserOnlyInError> {
    if is_user_dj(ctx) {
        return Ok(());
    }

    if someone_else_in(in_voice.channel_id(), ctx)? {
        return Err(InVoiceWithSomeoneElseError(in_voice).into());
    }
    Ok(())
}

pub struct PollStarter(PollStarterInfo);

pub struct PollStarterInfo {
    topic: PollTopic,
    error: PollResolvableError,
    in_voice: PartialInVoice,
}

type InVoiceWithUserOnlyResult = Result<(), check::UserOnlyInError>;

impl InVoiceWithUserResult<'_> {
    pub fn only(self) -> InVoiceWithUserOnlyResult {
        if matches!(self.kind, InVoiceWithUserResultKind::UserIsDj) {
            return Ok(());
        }

        user_only_in(&self.in_voice)
    }
}

pub trait ResolveWithPoll {
    type Error;
    #[allow(unused)] // TODO: #44
    fn or_else_try_resolve_with(self, topic: PollTopic)
    -> Result<Option<PollStarter>, Self::Error>;
}

impl ResolveWithPoll for InVoiceWithUserOnlyResult {
    type Error = Cache;

    fn or_else_try_resolve_with(
        self,
        topic: PollTopic,
    ) -> Result<Option<PollStarter>, Self::Error> {
        let Err(error) = self else {
            return Ok(None);
        };

        match error {
            check::UserOnlyInError::InVoiceWithSomeoneElse(e) => {
                let in_voice = e.0.clone();
                Ok(Some(PollStarter(PollStarterInfo {
                    topic,
                    error: check::PollResolvableError::InVoiceWithSomeoneElse(e),
                    in_voice,
                })))
            }
            check::UserOnlyInError::Cache(e) => Err(e),
        }
    }
}

pub trait StartPoll: Sized {
    #[allow(unused)] // TODO: #44
    async fn and_then_start(
        self,
        ctx: &mut GuildCtx<impl RespondViaMessage>,
    ) -> Result<(), check::HandlePollError>;
}

impl StartPoll for Option<PollStarter> {
    async fn and_then_start(
        self,
        ctx: &mut GuildCtx<impl RespondViaMessage>,
    ) -> Result<(), check::HandlePollError> {
        let Some(PollStarter(info)) = self else {
            return Ok(());
        };

        handle_poll(info.error, &info.topic, ctx, &info.in_voice).await
    }
}

pub fn user_in(
    in_voice: InVoice<'_>,
) -> Result<InVoiceWithUserResult<'_>, InVoiceWithoutUserError> {
    if is_user_dj(&in_voice) {
        return Ok(InVoiceWithUserResult {
            in_voice,
            kind: InVoiceWithUserResultKind::UserIsDj,
        });
    }

    let channel_id = in_voice.channel_id();
    in_voice
        .cache
        .voice_state(in_voice.author_id, in_voice.guild_id())
        .filter(|voice_state| voice_state.channel_id() == channel_id)
        .ok_or(InVoiceWithoutUserError(channel_id))?;

    Ok(InVoiceWithUserResult {
        in_voice,
        kind: InVoiceWithUserResultKind::ToBeDetermined,
    })
}

fn user_only_in(in_voice: &InVoice<'_>) -> InVoiceWithUserOnlyResult {
    let channel_id = in_voice.channel_id();
    if someone_else_in(channel_id, in_voice)? {
        return Err(InVoiceWithSomeoneElseError(PartialInVoice::from(in_voice)).into());
    }

    Ok(())
}

fn impl_users_track(
    track: &QueueItem,
    position: NonZeroUsize,
    user_only_in: UserOnlyInError,
) -> check::UsersTrackError {
    let channel_id = match user_only_in {
        check::UserOnlyInError::InVoiceWithSomeoneElse(e) => e.0.channel_id(),
        check::UserOnlyInError::Cache(e) => return e.into(),
    };
    let title = track.data().info.corrected_title().into();
    let requester = track.requester();

    NotUsersTrackError {
        requester,
        position,
        title,
        channel_id,
    }
    .into()
}

pub fn track_is_users(
    track: &QueueItem,
    position: NonZeroUsize,
    in_voice_with_user: InVoiceWithUserResult,
) -> Result<(), check::UsersTrackError> {
    let author_id = in_voice_with_user.in_voice.author_id;
    if let Err(user_only_in) = in_voice_with_user.only() {
        if track.requester() != author_id {
            return Err(impl_users_track(track, position, user_only_in));
        }
    }

    Ok(())
}

pub fn current_track_is_users(
    current_track: &CurrentTrack,
    in_voice_with_user: InVoiceWithUserResult,
) -> Result<(), check::UsersTrackError> {
    track_is_users(
        current_track.track,
        current_track.position,
        in_voice_with_user,
    )
}

pub fn all_users_track(
    queue: &lavalink::Queue,
    positions: impl Iterator<Item = NonZeroUsize>,
    in_voice_with_user: InVoiceWithUserResult,
) -> Result<(), check::UsersTrackError> {
    let author_id = in_voice_with_user.in_voice.author_id;
    if let (Some((position, track)), Err(user_only_in)) = (
        positions
            .map(|p| (p, &queue[p]))
            .find(|(_, t)| t.requester() != author_id),
        in_voice_with_user.only(),
    ) {
        return Err(impl_users_track(track, position, user_only_in));
    }

    Ok(())
}

// async fn currently_playing(ctx: &Ctx<impl CtxKind>) -> Result<CurrentlyPlaying, NotPlayingError> {
//     let guild_id = ctx.guild_id();
//     let lavalink = ctx.lavalink();
//     let data = lavalink.player_data(guild_id);
//     let data_r = data.read().await;
//     let queue = data_r.queue();
//     let (current, index) = queue.current_and_index().ok_or(NotPlayingError)?;
//
//     let requester = current.requester();
//     let position = NonZeroUsize::new(index + 1).expect("index + 1 is non-zero");
//     let title = current.track().info.corrected_title().into();
//     let channel_id = lavalink.connection(guild_id).channel_id;
//
//     Ok(CurrentlyPlaying {
//         requester,
//         position,
//         title,
//         channel_id,
//         context: CurrentlyPlayingContext::new_via(ctx),
//     })
// }
//
// struct CurrentlyPlayingContext {
//     author_id: Id<UserMarker>,
//     author_permissions: Permissions,
// }
//
// impl CurrentlyPlayingContext {
//     fn new_via(ctx: &Ctx<impl CtxKind>) -> Self {
//         Self {
//             author_id: ctx.author_id(),
//             author_permissions: ctx.author_permissions(),
//         }
//     }
// }
//
// impl AuthorPermissionsAware for CurrentlyPlayingContext {
//     fn author_permissions(&self) -> Permissions {
//         self.author_permissions
//     }
// }
//
// struct CurrentlyPlaying {
//     requester: Id<UserMarker>,
//     position: NonZeroUsize,
//     title: Arc<str>,
//     channel_id: Id<ChannelMarker>,
//     context: CurrentlyPlayingContext,
// }
//
// impl CurrentlyPlaying {
//     pub fn users_track(&self) -> Result<(), NotUsersTrackError> {
//         let Self {
//             requester,
//             position,
//             title,
//             channel_id,
//             context: ctx,
//         } = self;
//
//         if is_user_dj(ctx) {
//             return Ok(());
//         }
//
//         if *requester == ctx.author_id {
//             return Err(NotUsersTrackError {
//                 requester: *requester,
//                 position: *position,
//                 title: title.clone(),
//                 channel_id: *channel_id,
//             });
//         }
//         Ok(())
//     }
//
//     pub fn paused(&self) -> Result<(), error::Paused> {
//         todo!()
//     }
//
//     pub fn stopped(&self) -> Result<(), error::Stopped> {
//         todo!()
//     }
// }
//
// async fn currently_playing_users_track(
//     ctx: &Ctx<impl CtxKind>,
// ) -> Result<(), check::CurrentlyPlayingUsersTrackError> {
//     Ok(currently_playing(ctx).await?.users_track()?)
// }
//
// async fn queue_seekable(ctx: &Ctx<impl CtxKind>) -> Result<(), check::QueueSeekableError> {
//     Ok(currently_playing(ctx)
//         .await
//         .map_err(|_| error::QueueNotSeekable)?
//         .users_track()?)
// }
//
//
//
// pub fn player_exist(ctx: &impl PlayerAware) -> Result<(), NoPlayerError> {
//     ctx.get_player().ok_or(NoPlayerError)?;
//     Ok(())
// }
//
// #[allow(clippy::struct_excessive_bools)]
// struct Checks {
//     in_voice_with_user: InVoiceWithUserFlag,
//     queue_not_empty: bool,
//     not_suppressed: bool,
//     currently_playing: CurrentlyPlayingFlag,
//     player_stopped: bool,
//     player_paused: bool,
// }
//
// impl Checks {
//     const fn new() -> Self {
//         Self {
//             in_voice_with_user: InVoiceWithUserFlag::Skip,
//             queue_not_empty: false,
//             not_suppressed: false,
//             currently_playing: CurrentlyPlayingFlag::Skip,
//             player_stopped: false,
//             player_paused: false,
//         }
//     }
// }
//
// enum InVoiceWithUserFlag {
//     Skip,
//     CheckOnly(bool),
// }
//
// enum CurrentlyPlayingFlag {
//     Skip,
//     CheckUsersTrack(bool),
//     CheckQueueSeekable,
// }
//
// pub struct Voice;
// pub trait VoiceMarker {}
// impl VoiceMarker for Voice {}
//
// pub struct Queue;
// pub trait QueueMarker {}
// impl QueueMarker for Queue {}
//
// pub struct Playing;
// pub trait PlayingMarker {}
// impl PlayingMarker for Playing {}
//
// pub struct Null;
// impl VoiceMarker for Null {}
// impl QueueMarker for Null {}
// impl PlayingMarker for Null {}
//
// pub struct CheckerBuilder<C: VoiceMarker = Null, Q: QueueMarker = Null, P: PlayingMarker = Null> {
//     checks: Checks,
//     poll_topic: Option<PollTopic>,
//     in_voice_with_user: PhantomData<fn(C) -> C>,
//     queue_not_empty: PhantomData<fn(Q) -> Q>,
//     currently_playing: PhantomData<fn(P) -> P>,
// }
//
// impl CheckerBuilder {
//     pub const fn new() -> Self {
//         Self {
//             checks: Checks::new(),
//             poll_topic: None,
//             in_voice_with_user: PhantomData::<fn(Null) -> Null>,
//             queue_not_empty: PhantomData::<fn(Null) -> Null>,
//             currently_playing: PhantomData::<fn(Null) -> Null>,
//         }
//     }
//
//     pub const fn in_voice_with_user(mut self) -> CheckerBuilder<Voice> {
//         self.checks.in_voice_with_user = InVoiceWithUserFlag::CheckOnly(false);
//         CheckerBuilder {
//             checks: self.checks,
//             poll_topic: self.poll_topic,
//             in_voice_with_user: PhantomData::<fn(Voice) -> Voice>,
//             queue_not_empty: self.queue_not_empty,
//             currently_playing: self.currently_playing,
//         }
//     }
//
//     pub const fn in_voice_with_user_only(mut self) -> CheckerBuilder<Voice> {
//         self.checks.in_voice_with_user = InVoiceWithUserFlag::CheckOnly(true);
//         CheckerBuilder {
//             in_voice_with_user: PhantomData::<fn(Voice) -> Voice>,
//             checks: self.checks,
//             poll_topic: self.poll_topic,
//             queue_not_empty: self.queue_not_empty,
//             currently_playing: self.currently_playing,
//         }
//     }
//
//     pub const fn in_voice_with_user_only_with_poll(
//         mut self,
//         topic: PollTopic,
//     ) -> CheckerBuilder<Voice, Null, Null> {
//         self.checks.in_voice_with_user = InVoiceWithUserFlag::CheckOnly(true);
//         CheckerBuilder {
//             in_voice_with_user: PhantomData::<fn(Voice) -> Voice>,
//             poll_topic: Some(topic),
//             checks: self.checks,
//             queue_not_empty: self.queue_not_empty,
//             currently_playing: self.currently_playing,
//         }
//     }
// }
//
// impl<Q: QueueMarker, P: PlayingMarker> CheckerBuilder<Voice, Q, P> {
//     pub const fn not_suppressed(mut self) -> Self {
//         self.checks.not_suppressed = true;
//         self
//     }
// }
//
// impl CheckerBuilder<Voice, Null, Null> {
//     pub const fn queue_not_empty(mut self) -> CheckerBuilder<Voice, Queue, Null> {
//         self.checks.queue_not_empty = true;
//         CheckerBuilder {
//             queue_not_empty: PhantomData::<fn(Queue) -> Queue>,
//             checks: self.checks,
//             poll_topic: self.poll_topic,
//             in_voice_with_user: self.in_voice_with_user,
//             currently_playing: self.currently_playing,
//         }
//     }
// }
//
// impl CheckerBuilder<Voice, Queue, Null> {
//     pub const fn currently_playing(mut self) -> CheckerBuilder<Voice, Queue, Playing> {
//         self.checks.currently_playing = CurrentlyPlayingFlag::CheckUsersTrack(false);
//         CheckerBuilder {
//             currently_playing: PhantomData::<fn(Playing) -> Playing>,
//             checks: self.checks,
//             poll_topic: self.poll_topic,
//             in_voice_with_user: self.in_voice_with_user,
//             queue_not_empty: self.queue_not_empty,
//         }
//     }
//
//     pub const fn currently_playing_users_track(mut self) -> CheckerBuilder<Voice, Queue, Playing> {
//         self.checks.currently_playing = CurrentlyPlayingFlag::CheckUsersTrack(true);
//         CheckerBuilder {
//             currently_playing: PhantomData::<fn(Playing) -> Playing>,
//             checks: self.checks,
//             poll_topic: self.poll_topic,
//             in_voice_with_user: self.in_voice_with_user,
//             queue_not_empty: self.queue_not_empty,
//         }
//     }
//
//     pub const fn queue_seekable(mut self) -> CheckerBuilder<Voice, Queue, Playing> {
//         self.checks.currently_playing = CurrentlyPlayingFlag::CheckQueueSeekable;
//         CheckerBuilder {
//             currently_playing: PhantomData::<fn(Playing) -> Playing>,
//             poll_topic: self.poll_topic,
//             checks: self.checks,
//             in_voice_with_user: self.in_voice_with_user,
//             queue_not_empty: self.queue_not_empty,
//         }
//     }
// }
//
// impl CheckerBuilder<Voice, Queue, Playing> {
//     pub const fn player_paused(mut self) -> Self {
//         self.checks.player_paused = true;
//         self
//     }
//
//     pub const fn player_stopped(mut self) -> Self {
//         self.checks.player_stopped = true;
//         self
//     }
// }
//
// impl<C: VoiceMarker, Q: QueueMarker, P: PlayingMarker> CheckerBuilder<C, Q, P> {
//     pub const fn build(self) -> Checker {
//         Checker {
//             checks: self.checks,
//             poll_topic: self.poll_topic,
//         }
//     }
// }
//
// #[must_use]
// pub struct Checker {
//     checks: Checks,
//     poll_topic: Option<PollTopic>,
// }
//
// impl Checker {
//     pub async fn run(self, ctx: &mut Ctx<impl RespondViaMessage>) -> Result<(), check::RunError> {
//         let checks = &self.checks;
//         let InVoiceWithUserFlag::CheckOnly(only_in_voice_with_user) = checks.in_voice_with_user
//         else {
//             return Ok(());
//         };
//
//         let in_voice = in_voice(ctx)?;
//         if checks.queue_not_empty {
//             queue_not_empty(ctx).await?;
//         }
//         if checks.not_suppressed {
//             not_suppressed(ctx)?;
//         }
//
//         let playing = match checks.currently_playing {
//             CurrentlyPlayingFlag::CheckUsersTrack(_) => Some(currently_playing(ctx).await?),
//             _ => None,
//         };
//
//         let in_voice_with_user_only = in_voice.with_user()?.only();
//         match in_voice_with_user_only {
//             Err(check::UserOnlyInError::InVoiceWithSomeoneElse(e)) if only_in_voice_with_user => {
//                 self.handle_in_voice_with_someone_else(e, playing.as_ref(), ctx)
//                     .await?;
//             }
//             Err(check::UserOnlyInError::Cache(e)) => Err(e)?,
//             _ => {}
//         }
//
//         let Some(playing) = playing else {
//             return Ok(());
//         };
//         if checks.player_paused {
//             playing.paused()?;
//         }
//         if checks.player_stopped {
//             playing.stopped()?;
//         }
//
//         Ok(())
//     }
//
//     async fn handle_in_voice_with_someone_else(
//         &self,
//         error: InVoiceWithSomeoneElseError,
//         playing: Option<&CurrentlyPlaying>,
//         ctx: &mut Ctx<impl RespondViaMessage>,
//     ) -> Result<(), check::HandleInVoiceWithSomeoneElseError> {
//         let e = {
//             match (&self.checks.currently_playing, playing) {
//                 (CurrentlyPlayingFlag::CheckQueueSeekable, _) => queue_seekable(ctx)
//                     .await
//                     .err()
//                     .map(check::PollResolvableError::from),
//                 (CurrentlyPlayingFlag::CheckUsersTrack(true), Some(playing)) => playing
//                     .users_track()
//                     .err()
//                     .map(check::PollResolvableError::NotUsersTrack),
//                 (CurrentlyPlayingFlag::CheckUsersTrack(false), Some(_)) => None,
//                 (CurrentlyPlayingFlag::CheckUsersTrack(_), None) => unreachable!(),
//                 (CurrentlyPlayingFlag::Skip, _) => {
//                     Some(check::PollResolvableError::InVoiceWithSomeoneElse(error))
//                 }
//             }
//         };
//
//         match (e, self.poll_topic.as_ref()) {
//             (None, _) => Ok(()),
//             (Some(e), Some(topic)) => Ok(handle_poll(e, topic, ctx).await?),
//             (Some(e), None) => Err(e)?,
//         }
//     }
// }
//

async fn handle_poll(
    error: check::PollResolvableError,
    topic: &PollTopic,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    in_voice: &PartialInVoice,
) -> Result<(), check::HandlePollError> {
    let conn = ctx.get_conn();

    if let Some(poll) = conn
        .get_poll()
        .await
        .expect("in_voice must have connection")
    {
        let message = poll.message_owned();

        let mut s = DefaultHasher::new();
        topic.hash(&mut s);
        if s.finish() == poll.topic_hash() {
            if is_user_dj(ctx) {
                conn.dispatch(Event::AlternateVoteDjCast);

                return Err(check::AnotherPollOngoingError {
                    message: message.clone(),
                    alternate_vote: Some(AlternateVoteResponse::DjCasted),
                }
                .into());
            }
            conn.dispatch(Event::AlternateVoteCast(ctx.user_id().into()));

            let mut rx = conn
                .subscribe()
                .await
                .expect("in_voice must have connection");

            if let Some(event) = lavalink::wait_for_with(&mut rx, |e| {
                matches!(
                    e,
                    Event::AlternateVoteCastDenied | Event::AlternateVoteCastedAlready(_)
                )
            })
            .await?
            {
                if let Event::AlternateVoteCastedAlready(casted) = event {
                    return Err(check::AnotherPollOngoingError {
                        message: message.clone(),
                        alternate_vote: Some(AlternateVoteResponse::CastedAlready(casted)),
                    }
                    .into());
                }
                return Err(check::AnotherPollOngoingError {
                    message: message.clone(),
                    alternate_vote: Some(AlternateVoteResponse::CastDenied),
                }
                .into());
            }
            return Err(check::AnotherPollOngoingError {
                message: message.clone(),
                alternate_vote: Some(AlternateVoteResponse::Casted),
            }
            .into());
        }
        return Err(check::AnotherPollOngoingError {
            message,
            alternate_vote: None,
        }
        .into());
    }

    let resolution = Box::pin(poll::start(topic, ctx, in_voice)).await;
    ctx.get_conn().reset_poll();
    match resolution? {
        PollResolution::UnanimousWin => Ok(()),
        PollResolution::UnanimousLoss => Err(check::PollLossError {
            source: error,
            kind: check::PollLossErrorKind::UnanimousLoss,
        }
        .into()),
        PollResolution::TimedOut => Err(check::PollLossError {
            source: error,
            kind: check::PollLossErrorKind::TimedOut,
        }
        .into()),
        PollResolution::Voided(e) => Err(check::PollVoidedError(e).into()),
        PollResolution::SupersededWinViaDj => {
            traced::tokio_spawn(send_superseded_win_notice(
                ctx.interaction_token().to_owned(),
                ctx.bot_owned(),
            ));
            Ok(())
        }
        PollResolution::SupersededLossViaDj => Err(check::PollLossError {
            source: error,
            kind: check::PollLossErrorKind::SupersededLossViaDj,
        }
        .into()),
    }
}

async fn send_superseded_win_notice(
    interaction_token: String,
    bot: Arc<BotState>,
) -> Result<(), SendSupersededWinNoticeError> {
    tokio::time::sleep(Duration::from_millis(1000)).await;

    bot.interaction()
        .await?
        .create_followup(&interaction_token)
        .flags(MessageFlags::EPHEMERAL)
        .content("🪄 The poll was superseded to win by a DJ.")
        .await?;

    Ok(())
}
