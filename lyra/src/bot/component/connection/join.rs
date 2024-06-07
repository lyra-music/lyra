use std::{borrow::Cow, fmt::Display, sync::Arc};

use chrono::Utc;
use twilight_gateway::Event;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    application::interaction::InteractionChannel,
    channel::ChannelType,
    gateway::payload::outgoing::UpdateVoiceState,
    guild::Permissions,
    id::{
        marker::{ChannelMarker, GuildMarker, MessageMarker},
        Id,
    },
};

use crate::bot::{
    command::{
        check,
        macros::{bad, cant, nope, note, note_fol, out, sus_fol},
        model::{BotSlashCommand, CtxKind, GuildCtx, RespondViaMessage},
        require::{self, InVoiceCachedVoiceState},
        SlashCtx,
    },
    component::connection::{start_inactivity_timeout, users_in_voice},
    core::{
        model::{
            AuthorIdAware, BotState, BotStateAware, CacheAware, HttpAware, OwnedBotStateAware,
        },
        r#const::connection::INACTIVITY_TIMEOUT_SECS,
        traced,
    },
    error::{
        self,
        component::connection::join::{
            AutoJoinError, ConnectToError, ConnectToNewError, DeleteEmptyVoiceNoticeError,
            Error as JoinError, GetUsersVoiceChannelError, HandleResponseError, ImplAutoJoinError,
            ImplConnectToError, ImplJoinError, Pfe,
        },
        Cache as CacheError, CommandResult, UserNotInVoice as UserNotInVoiceError,
    },
    gateway::{GuildIdAware, SenderAware},
    lavalink::{Connection, LavalinkAware},
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
    let voice_state = ctx
        .cache()
        .voice_state(ctx.author_id(), ctx.guild_id())
        .ok_or(UserNotInVoiceError)?;

    let channel_id = voice_state.channel_id();
    let voice = ctx.cache().channel(channel_id).ok_or(CacheError)?;
    let channel_type = voice.kind;
    let channel_parent_id = voice.parent_id;

    Ok((channel_id, channel_type, channel_parent_id))
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

fn check_user_is_stage_manager(
    channel_type: ChannelType,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(), error::UserNotStageManager> {
    if channel_type == ChannelType::GuildStageVoice {
        check::user_is_stage_manager(ctx)?;
    };
    Ok(())
}

async fn connect_to_new(
    channel_id: Id<ChannelMarker>,
    channel_type: ChannelType,
    channel_parent_id: Option<Id<ChannelMarker>>,
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<Response, ConnectToNewError> {
    check_user_is_stage_manager(channel_type, ctx)?;

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
    check_user_is_stage_manager(channel_type, ctx)?;

    let old_channel_id = ctx
        .current_voice_state()
        .map(|voice_state| {
            let old_channel_id = voice_state.channel_id();
            if old_channel_id == channel_id {
                Err(error::InVoiceAlready(channel_id))?;
            }

            check::noone_else_in(old_channel_id, ctx)?;

            Ok::<_, ConnectToError>(old_channel_id)
        })
        .transpose()?;

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
    if !ctx
        .bot_permissions_for(channel_id)?
        .contains(Permissions::CONNECT)
    {
        Err(error::ConnectionForbidden(channel_id))?;
    }

    check::user_allowed_to_use(channel_id, channel_parent_id, ctx).await?;

    let joined = JoinedChannel::new(channel_id, channel_type);

    let voice_is_empty = users_in_voice(ctx, channel_id).ok_or(CacheError)? == 0;

    let response = old_channel_id.map_or_else(
        || {
            let connection = Connection::new(channel_id, ctx.channel_id());
            connection.notify_change();
            ctx.lavalink().new_connection_with(guild_id, connection);
            Response::Joined {
                voice: joined,
                empty: voice_is_empty,
            }
        },
        |from| {
            let mut connection =
            // SAFETY: `old_channel_id` is of variant `Some`, meaning another connection exists,
            //         so `ctx.lavalink().get_connection_mut(guild_id).unwrap_unchecked()` is safe
            unsafe {
                ctx.lavalink()
                    .get_connection_mut(guild_id)
                    .unwrap_unchecked()
            };
            connection.channel_id = channel_id;
            connection.notify_change();
            Response::Moved {
                from,
                to: joined,
                empty: voice_is_empty,
            }
        },
    );

    ctx.sender()
        .command(&UpdateVoiceState::new(guild_id, channel_id, true, false))?;

    if joined.kind == JoinedChannelType::Stage {
        ctx.bot()
            .http()
            .update_current_user_voice_state(guild_id)
            .channel_id(channel_id)
            .request_to_speak_timestamp(&Utc::now().to_rfc3339())
            .await?;
    }

    tracing::debug!("guild {guild_id} {response}");
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

    bot.standby()
        .wait_for(ctx.guild_id, move |e: &Event| {
            let Event::VoiceStateUpdate(voice_state) = e else {
                return false;
            };
            voice_state.channel_id.is_some_and(|id| {
                id == ctx.channel_id
                    && users_in_voice(&ctx.bot, ctx.channel_id).is_some_and(|n| n >= 1)
            })
        })
        .await?;

    bot.interaction()
        .await?
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
    ctx: &mut GuildCtx<impl RespondViaMessage>,
) -> Result<InVoiceCachedVoiceState, HandleResponseError> {
    let (joined, empty) = match response {
        Response::Joined { voice, empty } => {
            let stage = matches!(voice.kind, JoinedChannelType::Stage);
            out!(stage_fmt(&format!("🖇️ {}", voice.id.mention()), stage), ?ctx);
            (voice, empty)
        }
        Response::Moved { from, to, empty } => {
            let stage = matches!(to.kind, JoinedChannelType::Stage);
            out!(
                stage_fmt(
                    &format!("️📎🖇️ ~~{}~~ ➜ __{}__", from.mention(), to.id.mention()),
                    stage,
                ),
                ?ctx
            );
            (to, empty)
        }
    };

    if empty {
        let text_channel_id = ctx.channel_id();
        let empty_voice_notice_txt = format!(
            "Joined an empty voice channel. The bot will automatically disconnects if no one else joins in <t:{}:R>.",
            Utc::now().timestamp() + i64::from(INACTIVITY_TIMEOUT_SECS)
        );

        traced::tokio_spawn(start_inactivity_timeout(
            super::InactivityTimeoutContext::new_via(ctx),
            joined.id,
            text_channel_id,
        ));

        let empty_voice_notice = note_fol!(empty_voice_notice_txt, ?ctx);
        let empty_voice_notice_message_id = empty_voice_notice.model().await?.id;
        traced::tokio_spawn(delete_empty_voice_notice(DeleteEmptyVoiceNotice::new(
            ctx,
            empty_voice_notice_message_id,
            joined.id,
        )));
    }

    let state = ctx.current_voice_state().ok_or(CacheError)?;
    let muted = state.mute();
    if muted {
        sus_fol!(
            "Currently server muted; Some features will be limited.",
            ?ctx
        );
    }
    Ok(state.into())
}

pub async fn auto(
    ctx: &mut GuildCtx<impl RespondViaMessage>,
) -> Result<InVoiceCachedVoiceState, AutoJoinError> {
    Ok(handle_response(impl_auto_join(ctx).await?, ctx).await?)
}

pub async fn join(
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    channel: Option<InteractionChannel>,
) -> Result<InVoiceCachedVoiceState, JoinError> {
    Ok(handle_response(impl_join(ctx, channel).await?, ctx).await?)
}

/// Joins a voice/stage channel
#[derive(CreateCommand, CommandModel)]
#[command(name = "join", dm_permission = false)]
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
                bad!("Please specify a voice channel, or join one.", ctx);
            }
            Pfe::UserNotStageManager(_) => {
                nope!("Only **Stage Managers** can use a stage channel.", ctx);
            }
            Pfe::UserNotAllowed(_) => {
                nope!("You are not allowed to use that channel.", ctx);
            }
            Pfe::InVoiceAlready(e) => {
                note!(format!("Already connected to {}.", e.0.mention()), ctx);
            }
            Pfe::Forbidden(e) => {
                cant!(
                    format!("Insufficient permissions to join {}.", e.0.mention()),
                    ctx
                );
            }
            Pfe::Other(e) => Err(e)?,
        }
    }
}
