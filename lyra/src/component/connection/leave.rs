use std::fmt::Display;

use twilight_gateway::error::ChannelError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    gateway::payload::outgoing::UpdateVoiceState,
    id::{Id, marker::ChannelMarker},
};

use crate::{
    LavalinkAndGuildIdAware, LavalinkAware,
    command::{
        SlashCtx, check,
        macros::{caut, out},
        model::{BotSlashCommand, CtxKind, GuildCtx},
        require,
    },
    core::model::HttpAware,
    error::{
        CommandResult,
        component::connection::leave::{self, DisconnectCleanupError},
    },
    gateway::{GuildIdAware, SenderAware},
    lavalink::{DelegateMethods, Event, UnwrappedData},
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

pub(super) async fn disconnect_cleanup(
    cx: &(impl HttpAware + GuildIdAware + LavalinkAware + Sync),
) -> Result<(), DisconnectCleanupError> {
    let guild_id = cx.guild_id();
    let lavalink = cx.lavalink();

    lavalink.handle_for(guild_id).dispatch(Event::QueueClear);
    if let Some(player_ctx) = lavalink.get_player_context(guild_id) {
        player_ctx
            .data_unwrapped()
            .write()
            .await
            .delete_now_playing_message(cx)
            .await;
    }
    lavalink.drop_connection(guild_id);
    lavalink.delete_player(guild_id).await?;

    Ok(())
}

async fn leave(ctx: &GuildCtx<impl CtxKind>) -> Result<LeaveResponse, leave::Error> {
    let guild_id = ctx.guild_id();

    let in_voice = require::in_voice(ctx)?;
    let channel_id = in_voice.channel_id();
    let conn = ctx.get_conn();
    conn.set_channel(channel_id);
    check::user_in(in_voice)?.only()?;
    conn.disable_vsu_handler().await?;
    disconnect_cleanup(ctx).await?;
    disconnect(ctx)?;

    let response = LeaveResponse(channel_id);
    tracing::info!("guild {guild_id} {response}");
    Ok(response)
}

/// Leaves the currently connected voice/stage channel and clears the queue.
#[derive(CreateCommand, CommandModel)]
#[command(name = "leave", dm_permission = false)]
pub struct Leave;

impl BotSlashCommand for Leave {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        match leave(&ctx).await {
            Ok(LeaveResponse(voice)) => {
                out!(format!("📎 ~~{}~~.", voice.mention()), ctx);
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
