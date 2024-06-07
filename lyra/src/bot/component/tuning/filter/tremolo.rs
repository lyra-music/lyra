use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{bad, out},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    component::tuning::{
        check_user_is_dj_and_require_unsuppressed_player, filter::SetTremolo, UpdateFilter,
    },
    error::CommandResult,
};

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "tremolo", desc = ".")]
pub enum Tremolo {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enable Tremolo: Quickly oscillates the playback volume, giving a shuddering effect.
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// Oscillate at what frequency? [in Hz.] (If not given, a reasonable default is used)
    #[command(min_value = 0)]
    frequency: Option<f64>, // default: 2.0 [https://github.com/lavalink-devs/Lavalink/blob/master/protocol/src/commonMain/kotlin/dev/arbjerg/lavalink/protocol/v4/filters.kt#L82]
    /// Oscillate by how much intensity? [0~1, excluding 0] (If not given, a reasonable default is used)
    #[command(min_value = 0, max_value = 1)]
    depth: Option<f64>, // default: 0.5 [https://github.com/lavalink-devs/Lavalink/blob/master/protocol/src/commonMain/kotlin/dev/arbjerg/lavalink/protocol/v4/filters.kt#L83]
}

impl BotSlashCommand for On {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let Some(update) = SetTremolo::new(self.frequency, self.depth) else {
            bad!("Both frequency and depth must not be zero.", ctx);
        };
        let settings = update.settings();

        player.update_filter(Some(update)).await?;
        out!(format!("🎸🟢 Enabled tremolo ({settings})"), ctx);
    }
}

/// Disable Tremolo
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotSlashCommand for Off {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        player.update_filter(None::<SetTremolo>).await?;
        out!("🎸🔴 Disabled tremolo", ctx);
    }
}
