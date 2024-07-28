use std::fmt::Display;

use twilight_gateway::error::ChannelError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    gateway::payload::outgoing::UpdateVoiceState,
    id::{marker::ChannelMarker, Id},
};

use crate::{
    command::{
        check,
        macros::{caut, out},
        model::{BotSlashCommand, CtxKind, GuildCtx},
        require, SlashCtx,
    },
    error::{
        component::connection::leave::{self, PreDisconnectCleanupError},
        CommandResult,
    },
    gateway::{GuildIdAware, SenderAware},
    lavalink::Event,
    LavalinkAware,
};

pub(super) struct LeaveResponse(pub(super) Id<ChannelMarker>);

impl Display for LeaveResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "left {}", self.0)
    }
}

pub(super) fn disconnect(cx: &(impl SenderAware + GuildIdAware)) -> Result<(), ChannelError> {
    cx.sender()
        .command(&UpdateVoiceState::new(cx.guild_id(), None, false, false))?;

    Ok(())
}

pub(super) async fn pre_disconnect_cleanup(
    cx: &(impl GuildIdAware + LavalinkAware + Sync),
) -> Result<(), PreDisconnectCleanupError> {
    let guild_id = cx.guild_id();
    let lavalink = cx.lavalink();

    if let Some(connection) = lavalink.get_connection(guild_id) {
        connection.dispatch(Event::QueueClear);
    };
    lavalink.drop_connection(guild_id);
    lavalink.delete_player(guild_id).await?;

    Ok(())
}

async fn leave(ctx: &GuildCtx<impl CtxKind>) -> Result<LeaveResponse, leave::Error> {
    let guild_id = ctx.guild_id();

    let in_voice = require::in_voice(ctx)?;
    let connection = ctx.lavalink().connection_from(&in_voice);
    let channel_id = in_voice.channel_id();
    check::user_in(in_voice)?.only()?;

    connection.notify_change();
    drop(connection);
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
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        match leave(&ctx).await {
            Ok(LeaveResponse(voice)) => {
                out!(format!("📎 ~~{}~~", voice.mention()), ctx);
            }
            Err(e) => match e.match_not_in_voice_into() {
                leave::NotInVoiceMatchedError::NotInVoice(_) => {
                    caut!("Not currently connected to a voice channel.", ctx);
                }
                leave::NotInVoiceMatchedError::Other(e) => Err(e.into()),
            },
        }
    }
}
