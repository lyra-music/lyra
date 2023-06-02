use std::fmt::Display;

use async_trait::async_trait;
use lyra_proc::{check, err, out};
use twilight_gateway::MessageSender;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_lavalink::{model::Destroy, Lavalink};
use twilight_mention::Mention;
use twilight_model::{
    gateway::payload::outgoing::UpdateVoiceState,
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
};

use crate::bot::commands::{
    checks,
    errors::Error,
    models::{Context, LyraCommand},
};

struct LeaveResponse(Id<ChannelMarker>);

impl Display for LeaveResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "left   {}", self.0)
    }
}

struct DisconnectContext<'a> {
    sender: &'a MessageSender,
    guild_id: Id<GuildMarker>,
}

impl<'a> From<&'a Context> for DisconnectContext<'a> {
    fn from(ctx: &'a Context) -> Self {
        Self {
            sender: ctx.bot().sender(),
            guild_id: ctx.guild_id_unchecked(),
        }
    }
}

fn disconnect(ctx: DisconnectContext) -> anyhow::Result<()> {
    ctx.sender
        .command(&UpdateVoiceState::new(ctx.guild_id, None, false, false))?;

    Ok(())
}

struct LavalinkCleanupContext<'a> {
    lavalink: &'a Lavalink,
    guild_id: Id<GuildMarker>,
}

impl<'a> From<&'a Context> for LavalinkCleanupContext<'a> {
    fn from(ctx: &'a Context) -> Self {
        Self {
            lavalink: ctx.lavalink(),
            guild_id: ctx.guild_id_unchecked(),
        }
    }
}

async fn lavalink_cleanup(ctx: LavalinkCleanupContext<'_>) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id;
    let Some(player) = ctx.lavalink.players().get(&guild_id) else {return Ok(())};
    player.send(Destroy::from(guild_id))?;

    Ok(())
}

async fn leave(ctx: &Context) -> anyhow::Result<LeaveResponse> {
    let guild_id = ctx.guild_id_unchecked();
    let Some(voice) = ctx.current_voice_state() else {
        return Err(Error::NotInVoice.into());
    };

    let channel_id = voice.channel_id();
    checks::noone_else_in_voice(ctx, channel_id)?;

    lavalink_cleanup(ctx.into()).await?;
    disconnect(ctx.into())?;

    let response = LeaveResponse(channel_id);
    tracing::info!("guild {guild_id} {response}");
    Ok(response)
}

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "leave",
    desc = "Leaves the currently connected voice/stage channel and clears the queue"
)]
pub struct Leave;

#[async_trait]
impl LyraCommand for Leave {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()> {
        check!(Guild);

        match leave(&ctx).await {
            Ok(LeaveResponse(voice)) => {
                out!(&format!("📎 ~~{}~~", voice.mention()));
            }
            Err(e) => match e.downcast()? {
                Error::NotInVoice => {
                    err!("❗ Not currently connected to a voice channel");
                }
                other => return Err(other.into()),
            },
        }
    }
}
