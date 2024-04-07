use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    marker::PhantomData,
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use twilight_cache_inmemory::{model::CachedVoiceState, Reference};
use twilight_model::{
    channel::{message::MessageFlags, ChannelType},
    guild::Permissions,
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
};

use crate::bot::{
    command::model::{Ctx, CtxKind},
    component::config::access::CalculatorBuilder,
    core::{
        model::{AuthorPermissionsAware, BotState, CacheAware, OwnedBotStateAware},
        traced,
    },
    error::{
        self,
        command::check::{
            self, AlternateVoteResponse, SendSupersededWinNoticeError, UserOnlyInError,
        },
        Cache as CacheError, InVoiceWithSomeoneElse as InVoiceWithSomeoneElseError,
        InVoiceWithoutUser as InVoiceWithoutUserError, NotInVoice as NotInVoiceError,
        NotPlaying as NotPlayingError, NotUsersTrack as NotUsersTrackError,
        Suppressed as SuppressedError, UserNotAccessManager as UserNotAccessManagerError,
        UserNotAllowed as UserNotAllowedError, UserNotDj as UserNotDjError,
        UserNotStageManager as UserNotStageManagerError,
    },
    gateway::{ExpectedGuildIdAware, GuildIdAware},
    lavalink::{self, CorrectTrackInfo, DelegateMethods, Event, LavalinkAware, QueueItem},
};

use super::{
    model::RespondViaMessage, poll, poll::Resolution as PollResolution, poll::Topic as PollTopic,
};

pub const DJ_PERMISSIONS: Permissions = Permissions::MOVE_MEMBERS.union(Permissions::MUTE_MEMBERS);
pub const ACCESS_MANAGER_PERMISSIONS: Permissions =
    Permissions::MANAGE_ROLES.union(Permissions::MANAGE_CHANNELS);

pub const STAGE_MANAGER_PERMISSIONS: Permissions = Permissions::MANAGE_CHANNELS
    .union(Permissions::MUTE_MEMBERS)
    .union(Permissions::MOVE_MEMBERS);

type InVoiceResult<'a> = Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedVoiceState>;

pub fn does_user_have_permissions(
    permissions: Permissions,
    ctx: &impl AuthorPermissionsAware,
) -> bool {
    let author_permissions = ctx.author_permissions();
    author_permissions.contains(permissions)
        || author_permissions.contains(Permissions::ADMINISTRATOR)
}

#[inline]
pub fn is_user_dj(ctx: &impl AuthorPermissionsAware) -> bool {
    does_user_have_permissions(DJ_PERMISSIONS, ctx)
}

pub fn user_is_dj(ctx: &impl AuthorPermissionsAware) -> Result<(), UserNotDjError> {
    if !is_user_dj(ctx) {
        return Err(UserNotDjError);
    }
    Ok(())
}

pub fn user_is_access_manager(
    ctx: &impl AuthorPermissionsAware,
) -> Result<(), UserNotAccessManagerError> {
    if !does_user_have_permissions(ACCESS_MANAGER_PERMISSIONS, ctx) {
        return Err(UserNotAccessManagerError);
    }
    Ok(())
}

pub fn user_is_stage_manager(
    ctx: &impl AuthorPermissionsAware,
) -> Result<(), UserNotStageManagerError> {
    if !does_user_have_permissions(STAGE_MANAGER_PERMISSIONS, ctx) {
        return Err(UserNotStageManagerError);
    }
    Ok(())
}

pub async fn user_allowed_in(ctx: &Ctx<impl CtxKind>) -> Result<(), check::UserAllowedError> {
    let Some(guild_id) = ctx.get_guild_id() else {
        return Ok(());
    };

    if user_is_access_manager(ctx).is_ok() {
        return Ok(());
    }

    let channel = ctx.channel();
    let mut access_calculator_builder = CalculatorBuilder::new(guild_id, ctx.db().clone())
        .user(ctx.author_id())
        .roles(ctx.member().roles.iter());
    match channel.kind {
        ChannelType::PublicThread
        | ChannelType::PrivateThread
        | ChannelType::AnnouncementThread => {
            let parent_id = channel
                .parent_id
                .expect("threads must have a parent channel");
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
    };

    let user_allowed_to_use_commands = access_calculator_builder.build().await?.calculate();
    if !user_allowed_to_use_commands {
        Err(UserNotAllowedError)?;
    }
    Ok(())
}

pub async fn user_allowed_to_use(
    channel_id: Id<ChannelMarker>,
    channel_parent_id: Option<Id<ChannelMarker>>,
    ctx: &Ctx<impl CtxKind>,
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
        Err(UserNotAllowedError)?;
    };
    Ok(())
}

pub fn in_voice<T: CtxKind>(ctx: &Ctx<T>) -> Result<InVoice<T>, NotInVoiceError> {
    let result = ctx.current_voice_state().ok_or(NotInVoiceError)?;
    Ok(InVoice { result, ctx })
}

pub struct InVoice<'a, T: CtxKind> {
    result: InVoiceResult<'a>,
    ctx: &'a Ctx<T>,
}

pub fn user_in<T: CtxKind>(
    channel_id: Id<ChannelMarker>,
    ctx: &Ctx<T>,
) -> Result<InVoiceWithUserResult<T>, InVoiceWithoutUserError> {
    if is_user_dj(ctx) {
        return Ok(InVoiceWithUserResult::Exempted);
    }

    let result = ctx
        .cache()
        .voice_state(ctx.author_id(), ctx.guild_id())
        .filter(|voice_state| voice_state.channel_id() == channel_id)
        .ok_or(InVoiceWithoutUserError(channel_id))?;

    Ok(InVoiceWithUserResult::ToBeDetermined(InVoiceWithUser {
        result,
        ctx,
    }))
}

fn someone_else_in(channel_id: Id<ChannelMarker>, ctx: &Ctx<impl CtxKind>) -> Option<bool> {
    ctx.cache()
        .voice_channel_states(channel_id)
        .map(|mut states| {
            states.any(|v| {
                !ctx.cache()
                    .user(v.user_id())
                    .expect("user of `v.user_id()` must exist in the cache")
                    .bot
                    && v.user_id() != ctx.author_id()
            })
        })
}

impl<'a, T: CtxKind> InVoice<'a, T> {
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.result.channel_id()
    }

    pub fn with_user(self) -> Result<InVoiceWithUserResult<'a, T>, InVoiceWithoutUserError> {
        if is_user_dj(self.ctx) {
            return Ok(InVoiceWithUserResult::Exempted);
        }

        let ctx = self.ctx;
        let channel_id = self.channel_id();
        let result = ctx
            .cache()
            .voice_state(ctx.author_id(), ctx.guild_id())
            .filter(|voice_state| voice_state.channel_id() == channel_id)
            .ok_or(InVoiceWithoutUserError(channel_id))?;

        Ok(InVoiceWithUserResult::ToBeDetermined(InVoiceWithUser {
            result,
            ctx,
        }))
    }

    pub fn with_someone_else(self) -> Result<(), check::InVoiceWithSomeoneElseError> {
        let channel_id = self.channel_id();

        if !someone_else_in(channel_id, self.ctx).ok_or(CacheError)? {
            Err(error::InVoiceWithoutSomeoneElse(channel_id))?;
        }
        Ok(())
    }
}

pub struct InVoiceWithUser<'a, T: CtxKind> {
    result: InVoiceResult<'a>,
    ctx: &'a Ctx<T>,
}

pub enum InVoiceWithUserResult<'a, T: CtxKind> {
    Exempted,
    ToBeDetermined(InVoiceWithUser<'a, T>),
}

pub fn noone_else_in(
    channel_id: Id<ChannelMarker>,
    ctx: &Ctx<impl CtxKind>,
) -> Result<(), check::UserOnlyInError> {
    if is_user_dj(ctx) {
        return Ok(());
    }

    if someone_else_in(channel_id, ctx).ok_or(CacheError)? {
        Err(InVoiceWithSomeoneElseError(channel_id))?;
    }
    Ok(())
}

impl<'a, T: CtxKind> InVoiceWithUserResult<'a, T> {
    pub fn only(self) -> Result<(), check::UserOnlyInError> {
        let InVoiceWithUserResult::ToBeDetermined(result) = self else {
            return Ok(());
        };
        let channel_id = result.result.channel_id();

        if someone_else_in(channel_id, result.ctx).ok_or(CacheError)? {
            Err(InVoiceWithSomeoneElseError(channel_id))?;
        }
        Ok(())
    }
}

pub fn in_voice_with_user<T: CtxKind>(
    ctx: &Ctx<T>,
) -> Result<InVoiceWithUserResult<'_, T>, check::InVoiceWithUserError> {
    Ok(in_voice(ctx)?.with_user()?)
}

pub fn in_voice_with_user_only<T: CtxKind>(
    ctx: &Ctx<T>,
) -> Result<(), check::InVoiceWithUserOnlyError> {
    Ok(in_voice(ctx)?.with_user()?.only()?)
}

pub fn not_suppressed(ctx: &Ctx<impl CtxKind>) -> Result<(), check::NotSuppressedError> {
    let voice_state = ctx.current_voice_state().ok_or(CacheError)?;
    let voice_state_channel = ctx
        .cache()
        .channel(voice_state.channel_id())
        .ok_or(CacheError)?;

    if voice_state.mute() {
        Err(SuppressedError::Muted)?;
    }
    let speaker_in_stage =
        voice_state.suppress() && matches!(voice_state_channel.kind, ChannelType::GuildStageVoice);
    drop(voice_state);

    if speaker_in_stage {
        Err(SuppressedError::NotSpeaker)?;
    }
    Ok(())
}

pub async fn queue_not_empty(ctx: &Ctx<impl CtxKind>) -> Result<(), error::QueueEmpty> {
    let guild_id = ctx.guild_id();
    if ctx
        .lavalink()
        .player_data(guild_id)
        .read()
        .await
        .queue()
        .is_empty()
    {
        return Err(error::QueueEmpty);
    }

    Ok(())
}

async fn currently_playing(ctx: &Ctx<impl CtxKind>) -> Result<CurrentlyPlaying, NotPlayingError> {
    let guild_id = ctx.guild_id();
    let data = ctx.lavalink().player_data(guild_id);
    let data_r = data.read().await;
    let queue = data_r.queue();
    let (current, index) = queue.current_and_index().ok_or(NotPlayingError)?;

    let requester = current.requester();
    let position = NonZeroUsize::new(index + 1).expect("`index + 1` must be nonzero");
    let title = current.track().info.corrected_title().into();
    let channel_id = ctx.lavalink().connection(guild_id).channel_id;

    Ok(CurrentlyPlaying {
        requester,
        position,
        title,
        channel_id,
        context: CurrentlyPlayingContext::new_via(ctx),
    })
}

struct CurrentlyPlayingContext {
    author_id: Id<UserMarker>,
    author_permissions: Permissions,
}

impl CurrentlyPlayingContext {
    fn new_via(ctx: &Ctx<impl CtxKind>) -> Self {
        Self {
            author_id: ctx.author_id(),
            author_permissions: ctx.author_permissions(),
        }
    }
}

impl AuthorPermissionsAware for CurrentlyPlayingContext {
    fn author_permissions(&self) -> Permissions {
        self.author_permissions
    }
}

struct CurrentlyPlaying {
    requester: Id<UserMarker>,
    position: NonZeroUsize,
    title: Arc<str>,
    channel_id: Id<ChannelMarker>,
    context: CurrentlyPlayingContext,
}

impl CurrentlyPlaying {
    pub fn users_track(&self) -> Result<(), NotUsersTrackError> {
        let Self {
            requester,
            position,
            title,
            channel_id,
            context: ctx,
        } = self;

        if is_user_dj(ctx) {
            return Ok(());
        }

        if *requester == ctx.author_id {
            return Err(NotUsersTrackError {
                requester: *requester,
                position: *position,
                title: title.clone(),
                channel_id: *channel_id,
            });
        }
        Ok(())
    }

    pub fn paused(&self) -> Result<(), error::Paused> {
        todo!()
    }

    pub fn stopped(&self) -> Result<(), error::Stopped> {
        todo!()
    }
}

async fn currently_playing_users_track(
    ctx: &Ctx<impl CtxKind>,
) -> Result<(), check::CurrentlyPlayingUsersTrackError> {
    Ok(currently_playing(ctx).await?.users_track()?)
}

async fn queue_seekable(ctx: &Ctx<impl CtxKind>) -> Result<(), check::QueueSeekableError> {
    Ok(currently_playing(ctx)
        .await
        .map_err(|_| error::QueueNotSeekable)?
        .users_track()?)
}

fn impl_users_track(
    position: NonZeroUsize,
    track: &QueueItem,
    user_only_in: UserOnlyInError,
) -> check::UsersTrackError {
    let channel_id = match user_only_in {
        check::UserOnlyInError::InVoiceWithSomeoneElse(e) => e.0,
        check::UserOnlyInError::Cache(e) => return e.into(),
    };
    let title = track.track().info.corrected_title().into();
    let requester = track.requester();

    NotUsersTrackError {
        requester,
        position,
        title,
        channel_id,
    }
    .into()
}

pub fn users_track<T: CtxKind>(
    position: NonZeroUsize,
    in_voice_with_user: InVoiceWithUserResult<T>,
    queue: &lavalink::Queue,
    ctx: &Ctx<T>,
) -> Result<(), check::UsersTrackError> {
    if let Err(e) = in_voice_with_user.only() {
        let track = &queue[position.get() - 1];
        if track.requester() != ctx.author_id() {
            return Err(impl_users_track(position, track, e));
        }
    }

    Ok(())
}

pub fn all_users_track<T: CtxKind>(
    positions: impl Iterator<Item = NonZeroUsize>,
    in_voice_with_user: InVoiceWithUserResult<T>,
    queue: &lavalink::Queue,
    ctx: &Ctx<T>,
) -> Result<(), check::UsersTrackError> {
    if let (Some((position, track)), Err(e)) = (
        positions
            .map(|p| (p, &queue[p.get() - 1]))
            .find(|(_, t)| t.requester() != ctx.author_id()),
        in_voice_with_user.only(),
    ) {
        return Err(impl_users_track(position, track, e));
    }

    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
struct Checks {
    in_voice_with_user: InVoiceWithUserFlag,
    queue_not_empty: bool,
    not_suppressed: bool,
    currently_playing: CurrentlyPlayingFlag,
    player_stopped: bool,
    player_paused: bool,
}

impl Checks {
    const fn new() -> Self {
        Self {
            in_voice_with_user: InVoiceWithUserFlag::Skip,
            queue_not_empty: false,
            not_suppressed: false,
            currently_playing: CurrentlyPlayingFlag::Skip,
            player_stopped: false,
            player_paused: false,
        }
    }
}

enum InVoiceWithUserFlag {
    Skip,
    CheckOnly(bool),
}

enum CurrentlyPlayingFlag {
    Skip,
    CheckUsersTrack(bool),
    CheckQueueSeekable,
}

pub struct Voice;
pub trait VoiceMarker {}
impl VoiceMarker for Voice {}

pub struct Queue;
pub trait QueueMarker {}
impl QueueMarker for Queue {}

pub struct Playing;
pub trait PlayingMarker {}
impl PlayingMarker for Playing {}

pub struct Null;
impl VoiceMarker for Null {}
impl QueueMarker for Null {}
impl PlayingMarker for Null {}

pub struct CheckerBuilder<C: VoiceMarker = Null, Q: QueueMarker = Null, P: PlayingMarker = Null> {
    checks: Checks,
    poll_topic: Option<PollTopic>,
    in_voice_with_user: PhantomData<fn(C) -> C>,
    queue_not_empty: PhantomData<fn(Q) -> Q>,
    currently_playing: PhantomData<fn(P) -> P>,
}

impl CheckerBuilder {
    pub const fn new() -> Self {
        Self {
            checks: Checks::new(),
            poll_topic: None,
            in_voice_with_user: PhantomData::<fn(Null) -> Null>,
            queue_not_empty: PhantomData::<fn(Null) -> Null>,
            currently_playing: PhantomData::<fn(Null) -> Null>,
        }
    }

    pub const fn in_voice_with_user(mut self) -> CheckerBuilder<Voice> {
        self.checks.in_voice_with_user = InVoiceWithUserFlag::CheckOnly(false);
        CheckerBuilder {
            checks: self.checks,
            poll_topic: self.poll_topic,
            in_voice_with_user: PhantomData::<fn(Voice) -> Voice>,
            queue_not_empty: self.queue_not_empty,
            currently_playing: self.currently_playing,
        }
    }

    pub const fn in_voice_with_user_only(mut self) -> CheckerBuilder<Voice> {
        self.checks.in_voice_with_user = InVoiceWithUserFlag::CheckOnly(true);
        CheckerBuilder {
            in_voice_with_user: PhantomData::<fn(Voice) -> Voice>,
            checks: self.checks,
            poll_topic: self.poll_topic,
            queue_not_empty: self.queue_not_empty,
            currently_playing: self.currently_playing,
        }
    }

    pub const fn in_voice_with_user_only_with_poll(
        mut self,
        topic: PollTopic,
    ) -> CheckerBuilder<Voice, Null, Null> {
        self.checks.in_voice_with_user = InVoiceWithUserFlag::CheckOnly(true);
        CheckerBuilder {
            in_voice_with_user: PhantomData::<fn(Voice) -> Voice>,
            poll_topic: Some(topic),
            checks: self.checks,
            queue_not_empty: self.queue_not_empty,
            currently_playing: self.currently_playing,
        }
    }
}

impl<Q: QueueMarker, P: PlayingMarker> CheckerBuilder<Voice, Q, P> {
    pub const fn not_suppressed(mut self) -> Self {
        self.checks.not_suppressed = true;
        self
    }
}

impl CheckerBuilder<Voice, Null, Null> {
    pub const fn queue_not_empty(mut self) -> CheckerBuilder<Voice, Queue, Null> {
        self.checks.queue_not_empty = true;
        CheckerBuilder {
            queue_not_empty: PhantomData::<fn(Queue) -> Queue>,
            checks: self.checks,
            poll_topic: self.poll_topic,
            in_voice_with_user: self.in_voice_with_user,
            currently_playing: self.currently_playing,
        }
    }
}

impl CheckerBuilder<Voice, Queue, Null> {
    pub const fn currently_playing(mut self) -> CheckerBuilder<Voice, Queue, Playing> {
        self.checks.currently_playing = CurrentlyPlayingFlag::CheckUsersTrack(false);
        CheckerBuilder {
            currently_playing: PhantomData::<fn(Playing) -> Playing>,
            checks: self.checks,
            poll_topic: self.poll_topic,
            in_voice_with_user: self.in_voice_with_user,
            queue_not_empty: self.queue_not_empty,
        }
    }

    pub const fn currently_playing_users_track(mut self) -> CheckerBuilder<Voice, Queue, Playing> {
        self.checks.currently_playing = CurrentlyPlayingFlag::CheckUsersTrack(true);
        CheckerBuilder {
            currently_playing: PhantomData::<fn(Playing) -> Playing>,
            checks: self.checks,
            poll_topic: self.poll_topic,
            in_voice_with_user: self.in_voice_with_user,
            queue_not_empty: self.queue_not_empty,
        }
    }

    pub const fn queue_seekable(mut self) -> CheckerBuilder<Voice, Queue, Playing> {
        self.checks.currently_playing = CurrentlyPlayingFlag::CheckQueueSeekable;
        CheckerBuilder {
            currently_playing: PhantomData::<fn(Playing) -> Playing>,
            poll_topic: self.poll_topic,
            checks: self.checks,
            in_voice_with_user: self.in_voice_with_user,
            queue_not_empty: self.queue_not_empty,
        }
    }
}

impl CheckerBuilder<Voice, Queue, Playing> {
    pub const fn player_paused(mut self) -> Self {
        self.checks.player_paused = true;
        self
    }

    pub const fn player_stopped(mut self) -> Self {
        self.checks.player_stopped = true;
        self
    }
}

impl<C: VoiceMarker, Q: QueueMarker, P: PlayingMarker> CheckerBuilder<C, Q, P> {
    pub const fn build(self) -> Checker {
        Checker {
            checks: self.checks,
            poll_topic: self.poll_topic,
        }
    }
}

#[must_use]
pub struct Checker {
    checks: Checks,
    poll_topic: Option<PollTopic>,
}

impl Checker {
    pub async fn run(self, ctx: &mut Ctx<impl RespondViaMessage>) -> Result<(), check::RunError> {
        let checks = &self.checks;
        let InVoiceWithUserFlag::CheckOnly(only_in_voice_with_user) = checks.in_voice_with_user
        else {
            return Ok(());
        };

        let in_voice = in_voice(ctx)?;
        if checks.queue_not_empty {
            queue_not_empty(ctx).await?;
        }
        if checks.not_suppressed {
            not_suppressed(ctx)?;
        }

        let playing = match checks.currently_playing {
            CurrentlyPlayingFlag::CheckUsersTrack(_) => Some(currently_playing(ctx).await?),
            _ => None,
        };

        let in_voice_with_user_only = in_voice.with_user()?.only();
        match in_voice_with_user_only {
            Err(check::UserOnlyInError::InVoiceWithSomeoneElse(e)) if only_in_voice_with_user => {
                self.handle_in_voice_with_someone_else(e, playing.as_ref(), ctx)
                    .await?;
            }
            Err(check::UserOnlyInError::Cache(e)) => Err(e)?,
            _ => {}
        }

        let Some(playing) = playing else {
            return Ok(());
        };
        if checks.player_paused {
            playing.paused()?;
        }
        if checks.player_stopped {
            playing.stopped()?;
        }

        Ok(())
    }

    async fn handle_in_voice_with_someone_else(
        &self,
        error: InVoiceWithSomeoneElseError,
        playing: Option<&CurrentlyPlaying>,
        ctx: &mut Ctx<impl RespondViaMessage>,
    ) -> Result<(), check::HandleInVoiceWithSomeoneElseError> {
        let e = {
            match (&self.checks.currently_playing, playing) {
                (CurrentlyPlayingFlag::CheckQueueSeekable, _) => queue_seekable(ctx)
                    .await
                    .map_err(check::PollResolvableError::from)
                    .err(),
                (CurrentlyPlayingFlag::CheckUsersTrack(true), Some(playing)) => playing
                    .users_track()
                    .map_err(check::PollResolvableError::NotUsersTrack)
                    .err(),
                (CurrentlyPlayingFlag::CheckUsersTrack(_), Some(_)) => None,
                (CurrentlyPlayingFlag::CheckUsersTrack(_), None) => unreachable!(),
                (CurrentlyPlayingFlag::Skip, _) => {
                    Some(check::PollResolvableError::InVoiceWithSomeoneElse(error))
                }
            }
        };

        match (e, self.poll_topic.as_ref()) {
            (None, _) => Ok(()),
            (Some(e), Some(topic)) => Ok(handle_poll(e, topic, ctx).await?),
            (Some(e), None) => Err(e)?,
        }
    }
}

async fn handle_poll(
    error: check::PollResolvableError,
    topic: &PollTopic,
    ctx: &mut Ctx<impl RespondViaMessage>,
) -> Result<(), check::HandlePollError> {
    let guild_id = ctx.guild_id();
    let connection = ctx.lavalink().connection(guild_id);
    if let Some(poll) = connection.poll() {
        let message = poll.message_owned();

        let mut s = DefaultHasher::new();
        topic.hash(&mut s);
        if s.finish() == poll.topic_hash() {
            if is_user_dj(ctx) {
                connection.dispatch(Event::AlternateVoteDjCast);

                Err(check::AnotherPollOngoingError {
                    message: message.clone(),
                    alternate_vote: Some(AlternateVoteResponse::DjCasted),
                })?;
            }
            connection.dispatch(Event::AlternateVoteCast(ctx.author_id().into()));

            let mut rx = connection.subscribe();

            if let Some(event) = lavalink::wait_for_with(&mut rx, |e| {
                matches!(
                    e,
                    Event::AlternateVoteCastDenied | Event::AlternateVoteCastedAlready(_)
                )
            })
            .await?
            {
                if let Event::AlternateVoteCastedAlready(casted) = event {
                    Err(check::AnotherPollOngoingError {
                        message: message.clone(),
                        alternate_vote: Some(AlternateVoteResponse::CastedAlready(casted)),
                    })?;
                }
                Err(check::AnotherPollOngoingError {
                    message: message.clone(),
                    alternate_vote: Some(AlternateVoteResponse::CastDenied),
                })?;
            }
            Err(check::AnotherPollOngoingError {
                message: message.clone(),
                alternate_vote: Some(AlternateVoteResponse::Casted),
            })?;
        }
        Err(check::AnotherPollOngoingError {
            message,
            alternate_vote: None,
        })?;
    }

    drop(connection);
    let resolution = Box::pin(poll::start(topic, ctx)).await;
    ctx.lavalink().connection_mut(guild_id).reset_poll();
    match resolution? {
        PollResolution::UnanimousWin => Ok(()),
        PollResolution::UnanimousLoss => Err(check::PollLossError {
            source: error,
            kind: check::PollLossErrorKind::UnanimousLoss,
        })?,
        PollResolution::TimedOut => Err(check::PollLossError {
            source: error,
            kind: check::PollLossErrorKind::TimedOut,
        })?,
        PollResolution::Voided(e) => Err(check::PollVoidedError(e))?,
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
        })?,
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
        .content("🪄 The poll was superseded to win by a DJ.")?
        .await?;

    Ok(())
}
