use std::fmt::Display;

use twilight_gateway::error::SendError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    gateway::payload::outgoing::UpdateVoiceState,
    id::{marker::ChannelMarker, Id},
};

use crate::bot::{
    command::{
        check,
        macros::{caut, hid, out},
        model::{BotSlashCommand, Ctx, CtxKind},
        SlashCtx,
    },
    error::{
        command::Result as CommandResult,
        component::connection::leave::{self, PreDisconnectCleanupError},
    },
    gateway::{ExpectedGuildIdAware, SenderAware},
    lavalink::{self, LavalinkAware},
};

pub(super) struct LeaveResponse(pub(super) Id<ChannelMarker>);

impl Display for LeaveResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "left {}", self.0)
    }
}

pub(super) fn disconnect(ctx: &(impl SenderAware + ExpectedGuildIdAware)) -> Result<(), SendError> {
    ctx.sender()
        .command(&UpdateVoiceState::new(ctx.guild_id(), None, false, false))?;

    Ok(())
}

pub(super) async fn pre_disconnect_cleanup(
    ctx: &(impl ExpectedGuildIdAware + lavalink::LavalinkAware + Sync),
) -> Result<(), PreDisconnectCleanupError> {
    let guild_id = ctx.guild_id();
    let lavalink = ctx.lavalink();

    lavalink.dispatch_queue_clear(guild_id);
    lavalink.drop_connection(guild_id);
    lavalink.delete_player(guild_id).await?;

    Ok(())
}

async fn leave(ctx: &Ctx<impl CtxKind>) -> Result<LeaveResponse, leave::Error> {
    let guild_id = ctx.guild_id();

    let in_voice = check::in_voice(ctx)?;
    let channel_id = in_voice.channel_id();
    in_voice.with_user()?.only()?;

    ctx.lavalink().notify_connection_change(guild_id);
    pre_disconnect_cleanup(ctx).await?;
    disconnect(ctx)?;

    let response = LeaveResponse(channel_id);
    tracing::debug!("guild {guild_id} {response}");
    Ok(response)
}

/// Leaves the currently connected voice/stage channel and clears the queue
#[derive(CreateCommand, CommandModel)]
#[command(name = "leave", dm_permission = false)]
pub struct Leave;

impl BotSlashCommand for Leave {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        match leave(&ctx).await {
            Ok(LeaveResponse(voice)) => {
                out!(format!("📎 ~~{}~~", voice.mention()), ctx);
            }
            Err(e) => match e.match_not_in_voice_into() {
                leave::NotInVoiceMatchedError::NotInVoice(_) => {
                    caut!("Not currently connected to a voice channel.", ctx);
                }
                leave::NotInVoiceMatchedError::Other(e) => Err(e)?,
            },
        }
    }
}
