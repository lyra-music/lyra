use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        SlashCtx,
        macros::{bad, out},
        model::BotSlashCommand,
        require,
    },
    component::tuning::{
        UpdateFilter, check_user_is_dj_and_require_unsuppressed_player, filter::SetVibrato,
    },
    error::CommandResult,
};

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "vibrato", desc = ".")]
pub enum Vibrato {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enables Vibrato: Quickly oscillates the playback pitch, giving a shuddering effect.
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// Oscillate at what frequency? [in Hz. 0~14, excluding 0] (If not given, a reasonable default is used)
    #[command(min_value = 0, max_value = 14)]
    frequency: Option<f64>, // default: 2.0 [https://github.com/lavalink-devs/Lavalink/blob/master/protocol/src/commonMain/kotlin/dev/arbjerg/lavalink/protocol/v4/filters.kt#L88]
    /// Oscillate by how much intensity? [0~1, excluding 0] (If not given, a reasonable default is used)
    #[command(min_value = 0, max_value = 1)]
    depth: Option<f64>, // default: 0.5 [https://github.com/lavalink-devs/Lavalink/blob/master/protocol/src/commonMain/kotlin/dev/arbjerg/lavalink/protocol/v4/filters.kt#L89]
}

impl BotSlashCommand for On {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let Some(update) = SetVibrato::new(self.frequency, self.depth) else {
            bad!("Both frequency and depth must not be zero.", ctx);
        };
        let settings = update.settings();

        player.update_filter(Some(update)).await?;
        out!(format!("🎻🟢 Enabled vibrato ({settings})."), ctx);
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

        player.update_filter(None::<SetVibrato>).await?;
        out!("🎻🔴 Disabled vibrato.", ctx);
    }
}
