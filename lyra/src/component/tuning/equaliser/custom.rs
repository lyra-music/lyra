use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::GuildSlashCmdCtx,
    component::tuning::{
        UpdateFilter, equaliser::SetEqualiser, require_in_voice_unsuppressed_and_player,
    },
    core::model::response::initial::message::create::RespondWithMessage,
};

/// Enables the player equaliser with custom settings.
#[derive(CommandModel, CreateCommand)]
#[command(name = "custom")]
#[expect(clippy::struct_field_names)]
pub struct Custom {
    /// How much gain for band 1? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_1: Option<f64>,
    /// How much gain for band 2? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_2: Option<f64>,
    /// How much gain for band 3? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_3: Option<f64>,
    /// How much gain for band 4? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_4: Option<f64>,
    /// How much gain for band 5? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_5: Option<f64>,
    /// How much gain for band 6? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_6: Option<f64>,
    /// How much gain for band 7? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_7: Option<f64>,
    /// How much gain for band 8? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_8: Option<f64>,
    /// How much gain for band 9? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_9: Option<f64>,
    /// How much gain for band 10? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_10: Option<f64>,
    /// How much gain for band 11? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_11: Option<f64>,
    /// How much gain for band 12? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_12: Option<f64>,
    /// How much gain for band 13? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_13: Option<f64>,
    /// How much gain for band 14? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_14: Option<f64>,
    /// How much gain for band 15? [Default: 0, Muted: -0.25, Doubled: 0.25] (If not given, 0)
    #[command(min_value = -0.25, max_value = 1.0)]
    band_15: Option<f64>,
}

impl crate::command::model::BotGuildSlashCommand for Custom {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let equaliser = [
            self.band_1,
            self.band_2,
            self.band_3,
            self.band_4,
            self.band_5,
            self.band_6,
            self.band_7,
            self.band_8,
            self.band_9,
            self.band_10,
            self.band_11,
            self.band_12,
            self.band_13,
            self.band_14,
            self.band_15,
        ];

        let Some(filter) = SetEqualiser::new(equaliser) else {
            ctx.wrng(format!(
                "**At least one band gain must be changed**; Band gains must not all be `{}`.",
                SetEqualiser::DEFAULT_GAIN
            ))
            .await?;
            return Ok(());
        };

        player.update_filter(Some(filter)).await?;
        ctx.out("🎛️🟢 Enabled player equaliser (**`Custom Settings`**).")
            .await?;
        Ok(())
    }
}
