use lavalink_rs::model::player::{Distortion as LavalinkDistortion, Filters};
use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::{UpdateFilter, require_in_voice_unsuppressed_and_player},
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

struct SetDistortion(LavalinkDistortion);

impl SetDistortion {
    const DEFAULT_OFFSET: f64 = 0.;
    const DEFAULT_SCALE: f64 = 1.;

    fn new(distortion: LavalinkDistortion) -> Option<Self> {
        const ERR_MARGIN: f64 = f64::EPSILON;

        let offset_filter = |&o: &f64| (o - Self::DEFAULT_OFFSET).abs() > ERR_MARGIN;
        let scale_filter = |&s: &f64| (s - Self::DEFAULT_SCALE).abs() > ERR_MARGIN;

        let sin_offset = distortion.sin_offset.filter(offset_filter);
        let cos_offset = distortion.cos_offset.filter(offset_filter);
        let tan_offset = distortion.tan_offset.filter(offset_filter);
        let offset = distortion.offset.filter(offset_filter);

        let sin_scale = distortion.sin_scale.filter(scale_filter);
        let cos_scale = distortion.cos_scale.filter(scale_filter);
        let tan_scale = distortion.tan_scale.filter(scale_filter);
        let scale = distortion.scale.filter(scale_filter);

        [
            sin_offset, sin_scale, cos_offset, cos_scale, tan_offset, tan_scale, offset, scale,
        ]
        .iter()
        .any(Option::is_some)
        .then_some(Self(distortion))
    }
}

impl crate::component::tuning::ApplyFilter for Option<SetDistortion> {
    fn apply_to(self, filter: Filters) -> Filters {
        Filters {
            distortion: self.map(|d| d.0),
            ..filter
        }
    }
}

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "distortion", desc = ".")]
pub enum Distortion {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enables Distortion: If used correctly, can generate some rather unique audio effects.
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// What sin offset? (If not given, leave this setting unchanged)
    sin_offset: Option<f64>,
    /// What sin scale? (If not given, leave this setting unchanged)
    sin_scale: Option<f64>,
    /// What cos offset? (If not given, leave this setting unchanged)
    cos_offset: Option<f64>,
    /// What cos scale? (If not given, leave this setting unchanged)
    cos_scale: Option<f64>,
    /// What tan offset? (If not given, leave this setting unchanged)
    tan_offset: Option<f64>,
    /// What tan scale? (If not given, leave this setting unchanged)
    tan_scale: Option<f64>,
    /// What offset? (If not given, leave this setting unchanged)
    offset: Option<f64>,
    /// What scale? (If not given, leave this setting unchanged)
    scale: Option<f64>,
}

impl BotGuildSlashCommand for On {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let distortion = LavalinkDistortion {
            sin_offset: self.sin_offset,
            sin_scale: self.sin_scale,
            cos_offset: self.cos_offset,
            cos_scale: self.cos_scale,
            tan_offset: self.tan_offset,
            tan_scale: self.tan_scale,
            offset: self.offset,
            scale: self.scale,
        };
        let Some(update) = SetDistortion::new(distortion) else {
            ctx.wrng(
                format!(
                    "**At least one setting must be changed**: Offset settings must not all be `{}`, and scale settings must not all be `{}`.",
                    SetDistortion::DEFAULT_OFFSET,
                    SetDistortion::DEFAULT_SCALE,
                ),
            ).await?;
            return Ok(());
        };

        player.update_filter(Some(update)).await?;
        ctx.out("🍭🟢 Enabled distortion.").await?;
        Ok(())
    }
}

/// Disable Distortion
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotGuildSlashCommand for Off {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        player.update_filter(None::<SetDistortion>).await?;
        ctx.out("🍭🔴 Disabled distortion.").await?;
        Ok(())
    }
}
