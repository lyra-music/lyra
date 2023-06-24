use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use twilight_gateway::MessageSender;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    gateway::payload::outgoing::UpdateVoiceState,
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
};

use crate::bot::{
    commands::{
        checks,
        errors::Error,
        macros::{caut, hid, out},
        models::{App, LyraCommand},
        Context,
    },
    lavalink::{Lavalink, Lavalinkful},
};

pub(super) struct LeaveResponse(pub(super) Id<ChannelMarker>);

impl Display for LeaveResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "left   {}", self.0)
    }
}

pub(super) struct DisconnectContext<'a> {
    sender: &'a MessageSender,
    guild_id: Id<GuildMarker>,
}

impl<'a> DisconnectContext<'a> {
    pub(super) fn new(sender: &'a MessageSender, guild_id: Id<GuildMarker>) -> Self {
        Self { sender, guild_id }
    }
}

impl<'a> From<&'a Context<App>> for DisconnectContext<'a> {
    fn from(ctx: &'a Context<App>) -> Self {
        Self {
            sender: ctx.bot().sender(),
            guild_id: ctx.guild_id_unchecked(),
        }
    }
}

pub(super) fn disconnect(ctx: DisconnectContext) -> Result<()> {
    ctx.sender
        .command(&UpdateVoiceState::new(ctx.guild_id, None, false, false))?;

    Ok(())
}

pub(super) struct DestroyResourcesContext<'a> {
    lavalink: &'a Lavalink,
    guild_id: Id<GuildMarker>,
}

impl<'a> DestroyResourcesContext<'a> {
    pub(super) fn new(lavalink: &'a Lavalink, guild_id: Id<GuildMarker>) -> Self {
        Self { lavalink, guild_id }
    }
}

impl<'a> From<&'a Context<App>> for DestroyResourcesContext<'a> {
    fn from(ctx: &'a Context<App>) -> Self {
        Self {
            lavalink: ctx.lavalink(),
            guild_id: ctx.guild_id_unchecked(),
        }
    }
}

pub(super) async fn destroy_resources(ctx: DestroyResourcesContext<'_>) -> Result<()> {
    let guild_id = ctx.guild_id;
    ctx.lavalink.destroy_player(guild_id).await?;
    ctx.lavalink.remove_connection(guild_id);

    Ok(())
}

async fn leave(ctx: &Context<App>) -> Result<LeaveResponse> {
    let guild_id = ctx.guild_id_unchecked();
    let voice = ctx.current_voice_state().ok_or(Error::NotInVoice)?;

    let channel_id = voice.channel_id();
    checks::noone_else_in_voice(ctx, channel_id)?;

    ctx.lavalink().dispatch_connection_change(guild_id);
    destroy_resources(ctx.into()).await?;
    disconnect(ctx.into())?;

    let response = LeaveResponse(channel_id);
    tracing::info!("guild {guild_id} {response}");
    Ok(response)
}

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "leave",
    desc = "Leaves the currently connected voice/stage channel and clears the queue",
    dm_permission = false
)]
pub struct Leave;

#[async_trait]
impl LyraCommand for Leave {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        match leave(&ctx).await {
            Ok(LeaveResponse(voice)) => {
                out!(format!("📎 ~~{}~~", voice.mention()), ctx);
            }
            Err(e) => match e.downcast()? {
                Error::NotInVoice => {
                    caut!("Not currently connected to a voice channel.", ctx);
                }
                other => return Err(other.into()),
            },
        }
    }
}
