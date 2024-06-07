use lavalink_rs::model::player::{Filters, Rotation as LavalinkRotation};
use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{bad, out},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    component::tuning::{check_user_is_dj_and_require_unsuppressed_player, UpdateFilter},
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

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "rotation", desc = ".")]
pub enum Rotation {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enable Rotation (Audio Panning / "8D Audio"): Rotates the sound around the stereo channels.
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// Rotate at what frequency? [in Hz.]
    frequency: f64,
}

impl BotSlashCommand for On {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let Some(update) = SetRotation::new(self.frequency) else {
            bad!("Frequency must not be zero.", ctx);
        };
        let frequency = update.frequency();

        player.update_filter(Some(update)).await?;
        out!(
            format!("🍳🟢 Enabled rotation (Frequency: `{frequency} Hz.`)"),
            ctx
        );
    }
}

/// Disable Rotation
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotSlashCommand for Off {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        player.update_filter(None::<SetRotation>).await?;
        out!("🍳🔴 Disabled rotation", ctx);
    }
}
