use lavalink_rs::model::player::{ChannelMix as LavalinkChannelMix, Filters};
use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::{UpdateFilter, require_in_voice_unsuppressed_and_player},
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

struct SetChannelMix(LavalinkChannelMix);

impl SetChannelMix {
    const DEFAULT_SAME_CHANNEL: f64 = 1.;
    const DEFAULT_CROSS_CHANNEL: f64 = 0.;

    fn new(
        left_to_left: Option<f64>,
        left_to_right: Option<f64>,
        right_to_left: Option<f64>,
        right_to_right: Option<f64>,
    ) -> Option<Self> {
        const ERR_MARGIN: f64 = f64::EPSILON;

        let same_channel_filter = |&o: &f64| (o - Self::DEFAULT_SAME_CHANNEL).abs() > ERR_MARGIN;
        let cross_channel_filter = |&s: &f64| (s - Self::DEFAULT_CROSS_CHANNEL).abs() > ERR_MARGIN;

        let left_to_left = left_to_left.filter(same_channel_filter);
        let left_to_right = left_to_right.filter(cross_channel_filter);
        let right_to_left = right_to_left.filter(cross_channel_filter);
        let right_to_right = right_to_right.filter(same_channel_filter);

        [left_to_left, left_to_right, right_to_left, right_to_right]
            .iter()
            .any(Option::is_some)
            .then_some(Self(LavalinkChannelMix {
                left_to_left,
                left_to_right,
                right_to_left,
                right_to_right,
            }))
    }
}

impl crate::component::tuning::ApplyFilter for Option<SetChannelMix> {
    fn apply_to(self, filter: Filters) -> Filters {
        Filters {
            channel_mix: self.map(|c| c.0),
            ..filter
        }
    }
}

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "channel-mix", desc = ".")]
pub enum ChannelMix {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enables Channel Mix: Mixes both channels (left and right).
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// Keep the left channel by how much? [0~1] (If not given, leave this setting unchanged)
    #[command(min_value = 0, max_value = 1)]
    left_to_left: Option<f64>,
    /// Mix the left onto the right channel by how much? [0~1] (If not given, leave this setting unchanged)
    #[command(min_value = 0, max_value = 1)]
    left_to_right: Option<f64>,
    /// Mix the right onto the left channel by how much? [0~1] (If not given, leave this setting unchanged)
    #[command(min_value = 0, max_value = 1)]
    right_to_left: Option<f64>,
    /// Keep the right channel by how much? [0~1] (If not given, leave this setting unchanged)
    #[command(min_value = 0, max_value = 1)]
    right_to_right: Option<f64>,
}

impl BotGuildSlashCommand for On {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let Some(update) = SetChannelMix::new(
            self.left_to_left,
            self.left_to_right,
            self.right_to_left,
            self.right_to_right,
        ) else {
            ctx.wrng(
                format!(
                    "**At least one setting must be changed**: Same-channel settings must not all be `{}`, and cross-channel settings must not all be `{}`.",
                    SetChannelMix::DEFAULT_SAME_CHANNEL,
                    SetChannelMix::DEFAULT_CROSS_CHANNEL,
                ),
            ).await?;
            return Ok(());
        };

        player.update_filter(Some(update)).await?;
        ctx.out("⚗️🟢 Enabled channel mix).").await?;
        Ok(())
    }
}

/// Disable Channel Mix
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotGuildSlashCommand for Off {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        player.update_filter(None::<SetChannelMix>).await?;
        ctx.out("⚗️🔴 Disabled channel mix.").await?;
        Ok(())
    }
}
