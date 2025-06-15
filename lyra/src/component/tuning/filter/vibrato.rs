use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::{
        UpdateFilter, check_user_is_dj_and_require_unsuppressed_player, filter::SetVibrato,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
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

impl BotGuildSlashCommand for On {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let Some(update) = SetVibrato::new(self.frequency, self.depth) else {
            ctx.wrng("Both frequency and depth must not be zero.")
                .await?;
            return Ok(());
        };
        let settings = update.settings();

        player.update_filter(Some(update)).await?;
        ctx.out(format!("🎻🟢 Enabled vibrato ({settings})."))
            .await?;
        Ok(())
    }
}

/// Disable Tremolo
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotGuildSlashCommand for Off {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        player.update_filter(None::<SetVibrato>).await?;
        ctx.out("🎻🔴 Disabled vibrato.").await?;
        Ok(())
    }
}
