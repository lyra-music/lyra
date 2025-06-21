use lavalink_rs::model::player::{Filters, Rotation as LavalinkRotation};
use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::{UpdateFilter, require_in_voice_unsuppressed_and_player},
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

struct SetRotation(LavalinkRotation);

impl SetRotation {
    fn new(frequency: f64) -> Option<Self> {
        (frequency != 0.).then_some(Self(LavalinkRotation {
            rotation_hz: Some(frequency),
        }))
    }

    fn frequency(&self) -> f64 {
        self.0.rotation_hz.unwrap_or_default()
    }
}

impl super::ApplyFilter for Option<SetRotation> {
    fn apply_to(self, filter: Filters) -> Filters {
        Filters {
            rotation: self.map(|r| r.0),
            ..filter
        }
    }
}

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "rotation", desc = ".")]
pub enum Rotation {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enables Rotation (Audio Panning / "8D Audio"): Rotates the sound around the stereo channels.
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// Rotate at what frequency? [in Hz.]
    frequency: f64,
}

impl BotGuildSlashCommand for On {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let Some(update) = SetRotation::new(self.frequency) else {
            ctx.wrng("Frequency must not be zero.").await?;
            return Ok(());
        };
        let frequency = update.frequency();

        player.update_filter(Some(update)).await?;
        ctx.out(format!(
            "🍳🟢 Enabled rotation (Frequency: `{frequency} Hz.`)"
        ))
        .await?;
        Ok(())
    }
}

/// Disable Rotation
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotGuildSlashCommand for Off {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        player.update_filter(None::<SetRotation>).await?;
        ctx.out("🍳🔴 Disabled rotation.").await?;
        Ok(())
    }
}
