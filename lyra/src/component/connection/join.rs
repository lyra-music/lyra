use std::{borrow::Cow, fmt::Display, sync::Arc};

use lyra_ext::{iso8601_time, pretty::flags_display::FlagsDisplay, unix_time};
use twilight_gateway::Event;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::{
    Mention,
    timestamp::{Timestamp, TimestampStyle},
};
use twilight_model::{
    application::interaction::InteractionChannel,
    channel::ChannelType,
    gateway::payload::outgoing::UpdateVoiceState,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, MessageMarker},
    },
};

use crate::{
    LavalinkAware,
    command::{
        SlashCtx, check,
        model::{BotSlashCommand, CtxKind, FollowupCtxKind, GuildCtx, RespondViaMessage},
        require::{self, InVoiceCachedVoiceState},
    },
    component::connection::{start_inactivity_timeout, users_in_voice},
    core::{
        r#const::connection::INACTIVITY_TIMEOUT,
        model::{
            BotState, BotStateAware, CacheAware, HttpAware, OwnedBotStateAware, UserIdAware,
            response::{
                either::RespondOrFollowup, followup::Followup,
                initial::message::create::RespondWithMessage,
            },
        },
        r#static::application,
        traced,
    },
    error::{
        self, Cache as CacheError, CommandResult, UserNotInVoice as UserNotInVoiceError,
        component::connection::join::{
            AutoJoinError, ConnectToError, ConnectToNewError, DeleteEmptyVoiceNoticeError,
            Error as JoinError, GetUsersVoiceChannelError, HandleResponseError, ImplAutoJoinError,
            ImplConnectToError, ImplJoinError, Pfe,
        },
    },
    gateway::{GuildIdAware, SenderAware},
    lavalink::Connection,
};

pub(super) enum Response {
    Joined {
        voice: JoinedChannel,
        empty: bool,
    },
    Moved {
        from: Id<ChannelMarker>,
        to: JoinedChannel,
        empty: bool,
    },
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Joined { voice, .. } => write!(f, "joined {voice}"),
            Self::Moved { from, to, .. } => write!(f, "moved {from} -> {to}"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct JoinedChannel {
    id: Id<ChannelMarker>,
    pub(super) kind: JoinedChannelType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum JoinedChannelType {
    Voice,
    Stage,
}

impl JoinedChannel {
    pub(super) fn new(id: Id<ChannelMarker>, kind: ChannelType) -> Self {
        let kind = match kind {
            ChannelType::GuildVoice => JoinedChannelType::Voice,
            ChannelType::GuildStageVoice => JoinedChannelType::Stage,
            _ => panic!("unknown channel type: {kind:?}"),
        };
        Self { id, kind }
    }
}

impl Display for JoinedChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            JoinedChannelType::Voice => write!(f, "voice {}", self.id),
            JoinedChannelType::Stage => write!(f, "stage {}", self.id),
        }
    }
}

type GetUsersVoiceChannelResult =
    Result<(Id<ChannelMarker>, ChannelType, Option<Id<ChannelMarker>>), GetUsersVoiceChannelError>;

fn get_users_voice_channel(ctx: &GuildCtx<impl CtxKind>) -> GetUsersVoiceChannelResult {
    let channel_id = ctx
        .cache()
        .voice_state(ctx.user_id(), ctx.guild_id())
        .ok_or(UserNotInVoiceError)?
        .channel_id();
    let voice = ctx.cache().channel(channel_id).ok_or(CacheError)?;
    Ok((channel_id, voice.kind, voice.parent_id))
}

async fn impl_join(
    ctx: &GuildCtx<impl CtxKind>,
    channel: Option<InteractionChannel>,
) -> Result<Response, ImplJoinError> {
    let (channel_id, channel_type, channel_parent_id) = match channel {
        Some(v) => (v.id, v.kind, v.parent_id),
        None => get_users_voice_channel(ctx)?,
    };

    Ok(connect_to(channel_id, channel_type, channel_parent_id, ctx).await?)
}

async fn impl_auto_join(ctx: &GuildCtx<impl CtxKind>) -> Result<Response, ImplAutoJoinError> {
    let (channel_id, channel_type, channel_parent_id) = get_users_voice_channel(ctx)?;

    Ok(connect_to_new(channel_id, channel_type, channel_parent_id, ctx).await?)
}

fn check_user_is_stage_moderator(
    channel_type: ChannelType,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(), error::UserNotStageModerator> {
    if channel_type == ChannelType::GuildStageVoice {
        check::user_is_stage_moderator(ctx)?;
    }
    Ok(())
}

async fn connect_to_new(
    channel_id: Id<ChannelMarker>,
    channel_type: ChannelType,
    channel_parent_id: Option<Id<ChannelMarker>>,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<Response, ConnectToNewError> {
    check_user_is_stage_moderator(channel_type, ctx)?;

    Ok(impl_connect_to(
        channel_id,
        channel_parent_id,
        channel_type,
        None,
        ctx.guild_id(),
        ctx,
    )
    .await?)
}

async fn connect_to(
    channel_id: Id<ChannelMarker>,
    channel_type: ChannelType,
    channel_parent_id: Option<Id<ChannelMarker>>,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<Response, ConnectToError> {
    check_user_is_stage_moderator(channel_type, ctx)?;

    let old_channel_id = match require::in_voice(ctx) {
        Ok(ref in_voice) => {
            let old_channel_id = in_voice.channel_id();
            if old_channel_id == channel_id {
                return Err(error::InVoiceAlready(channel_id).into());
            }

            check::noone_else_in(in_voice.into(), ctx)?;

            Some(old_channel_id)
        }
        Err(_) => None,
    };

    Ok(impl_connect_to(
        channel_id,
        channel_parent_id,
        channel_type,
        old_channel_id,
        ctx.guild_id(),
        ctx,
    )
    .await?)
}

async fn impl_connect_to(
    channel_id: Id<ChannelMarker>,
    channel_parent_id: Option<Id<ChannelMarker>>,
    channel_type: ChannelType,
    old_channel_id: Option<Id<ChannelMarker>>,
    guild_id: Id<GuildMarker>,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<Response, ImplConnectToError> {
    const JOIN_PERMISSIONS: Permissions = Permissions::VIEW_CHANNEL
        .union(Permissions::CONNECT)
        .union(Permissions::SPEAK);

    let perms = ctx.bot_permissions_for(channel_id)?;
    if !perms.contains(JOIN_PERMISSIONS) {
        return Err(error::ConnectionForbidden {
            channel_id,
            missing: JOIN_PERMISSIONS - perms,
        }
        .into());
    }

    check::user_allowed_to_use(channel_id, channel_parent_id, ctx).await?;

    let joined = JoinedChannel::new(channel_id, channel_type);

    let voice_is_empty = users_in_voice(ctx, channel_id).ok_or(CacheError)? == 0;

    let lavalink = ctx.lavalink();
    let response = old_channel_id.map_or_else(
        || {
            let connection = Connection::new(channel_id, ctx.channel_id());
            connection.disable_vsu_handler();
            lavalink.new_connection_with(guild_id, connection);
            Response::Joined {
                voice: joined,
                empty: voice_is_empty,
            }
        },
        |from| {
            let conn = lavalink.handle_for(guild_id);
            conn.set_channel(channel_id);
            conn.disable_vsu_handler();
            Response::Moved {
                from,
                to: joined,
                empty: voice_is_empty,
            }
        },
    );

    ctx.sender()
        .command(&UpdateVoiceState::new(guild_id, channel_id, true, false))?;

    if let Ok(player) = require::player(ctx) {
        if old_channel_id.is_some() {
            tracing::debug!("waiting for voice server update...");
            let _ = ctx
                .bot()
                .standby()
                .wait_for_event(move |e: &Event| {
                    if let Event::VoiceServerUpdate(v) = e {
                        v.guild_id == guild_id
                    } else {
                        false
                    }
                })
                .await;
            tracing::debug!("voice server update received");
            player.update_voice_channel(voice_is_empty).await?;
        }
    }

    if joined.kind == JoinedChannelType::Stage {
        ctx.bot()
            .http()
            .update_current_user_voice_state(guild_id)
            .channel_id(channel_id)
            .request_to_speak_timestamp(&iso8601_time())
            .await?;
    }

    tracing::info!("guild {guild_id} {response}");
    Ok(response)
}

struct DeleteEmptyVoiceNotice {
    message_id: Id<MessageMarker>,
    channel_id: Id<ChannelMarker>,
    guild_id: Id<GuildMarker>,
    interaction_token: Box<str>,
    bot: Arc<BotState>,
}

impl DeleteEmptyVoiceNotice {
    fn new(
        ctx: &GuildCtx<impl CtxKind>,
        message_id: Id<MessageMarker>,
        channel_id: Id<ChannelMarker>,
    ) -> Self {
        Self {
            message_id,
            channel_id,
            guild_id: ctx.guild_id(),
            interaction_token: ctx.interaction_token().to_string().into(),
            bot: ctx.bot_owned(),
        }
    }
}

async fn delete_empty_voice_notice(
    ctx: DeleteEmptyVoiceNotice,
) -> Result<(), DeleteEmptyVoiceNoticeError> {
    let bot = ctx.bot.clone();
    let bot_user_id = bot.user_id();
    let channel_id = ctx.channel_id;

    bot.standby()
        .wait_for(ctx.guild_id, move |e: &Event| {
            let Event::VoiceStateUpdate(voice_state) = e else {
                return false;
            };
            (voice_state.user_id == bot_user_id && voice_state.channel_id != Some(channel_id)) // bot changed channel
                || users_in_voice(&ctx.bot, channel_id).is_some_and(|n| n >= 1) // bot not alone
        })
        .await?;

    bot.http()
        .interaction(application::id())
        .delete_followup(&ctx.interaction_token, ctx.message_id)
        .await?;
    Ok(())
}

#[inline]
fn stage_fmt(txt: &str, stage: bool) -> Cow<'_, str> {
    if stage {
        return Cow::Owned(String::from("🌠") + txt);
    }
    Cow::Borrowed(txt)
}

async fn handle_response(
    response: Response,
    ctx: &mut GuildCtx<impl RespondViaMessage + FollowupCtxKind>,
    send_muted_notice: bool,
) -> Result<InVoiceCachedVoiceState, HandleResponseError> {
    let (joined, empty) = match response {
        Response::Joined { voice, empty } => {
            let stage = matches!(voice.kind, JoinedChannelType::Stage);
            ctx.out_f(stage_fmt(&format!("🖇️ {}.", voice.id.mention()), stage))
                .await?;
            (voice, empty)
        }
        Response::Moved { from, to, empty } => {
            let stage = matches!(to.kind, JoinedChannelType::Stage);
            ctx.out_f(stage_fmt(
                &format!("️📎🖇️ ~~{}~~ ➜ __{}__.", from.mention(), to.id.mention()),
                stage,
            ))
            .await?;
            (to, empty)
        }
    };

    if empty {
        let duration = unix_time() + INACTIVITY_TIMEOUT;
        let timestamp = Timestamp::new(duration.as_secs(), Some(TimestampStyle::RelativeTime));
        let empty_voice_notice_txt = format!(
            "Joined an empty voice channel. The bot will automatically disconnects if no one else joins in {}.",
            timestamp.mention()
        );

        traced::tokio_spawn(start_inactivity_timeout(
            super::InactivityTimeoutContext::from(&*ctx),
            joined.id,
        ));

        let empty_voice_notice = ctx.note_f(empty_voice_notice_txt).await?;
        let empty_voice_notice_message_id = empty_voice_notice.retrieve_message().await?.id;
        traced::tokio_spawn(delete_empty_voice_notice(DeleteEmptyVoiceNotice::new(
            ctx,
            empty_voice_notice_message_id,
            joined.id,
        )));
    }

    let state = ctx.current_voice_state().ok_or(CacheError)?;
    if send_muted_notice && state.mute() {
        ctx.suspf("**Currently server muted**; Some features will be limited.")
            .await?;
    }
    Ok(state.into())
}

pub async fn auto(
    ctx: &mut GuildCtx<impl RespondViaMessage + FollowupCtxKind>,
) -> Result<InVoiceCachedVoiceState, AutoJoinError> {
    Ok(handle_response(impl_auto_join(ctx).await?, ctx, false).await?)
}

pub async fn join(
    ctx: &mut GuildCtx<impl RespondViaMessage + FollowupCtxKind>,
    channel: Option<InteractionChannel>,
) -> Result<InVoiceCachedVoiceState, JoinError> {
    Ok(handle_response(impl_join(ctx, channel).await?, ctx, true).await?)
}

/// Joins a voice/stage channel.
#[derive(CreateCommand, CommandModel)]
#[command(name = "join", contexts = "guild")]
pub struct Join {
    /// Which channel? (if not given, your currently connected channel)
    #[command(channel_types = "guild_voice guild_stage_voice")]
    channel: Option<InteractionChannel>,
}

impl BotSlashCommand for Join {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let Err(e) = join(&mut ctx, self.channel).await else {
            return Ok(());
        };

        match e.flatten_partially_into() {
            Pfe::UserNotInVoice(_) => {
                ctx.wrng("Please specify a voice channel, or join one.")
                    .await?;
                Ok(())
            }
            Pfe::UserNotStageModerator(_) => {
                ctx.nope("Only **Stage Moderators** can use a stage channel.")
                    .await?;
                Ok(())
            }
            Pfe::UserNotAllowed(_) => {
                ctx.nope("You are not allowed to use that channel.").await?;
                Ok(())
            }
            Pfe::InVoiceAlready(e) => {
                ctx.note(format!("Already connected to {}.", e.0.mention()))
                    .await?;
                Ok(())
            }
            Pfe::Forbidden(e) => {
                ctx.blck(format!(
                    "**Insufficient permissions to join {}**: Missing {} permissions.",
                    e.channel_id.mention(),
                    e.missing.pretty_display_code()
                ))
                .await?;
                Ok(())
            }
            Pfe::Other(e) => Err(e.into()),
        }
    }
}
