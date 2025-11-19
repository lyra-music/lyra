use lyra_ext::num::cast::usize_as_u8;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::GuildSlashCmdCtx,
    component::tuning::{
        UpdateFilter, equaliser::SetEqualiser, require_in_voice_unsuppressed_and_player,
    },
    core::model::response::initial::message::create::RespondWithMessage,
};

lyra_proc::read_equaliser_presets_as!(EqualiserPreset);

impl From<EqualiserPreset> for SetEqualiser {
    fn from(value: EqualiserPreset) -> Self {
        let gains = value.gains();
        Self(core::array::from_fn(|i| {
            lavalink_rs::model::player::Equalizer {
                band: usize_as_u8(i),
                gain: gains[i],
            }
        }))
    }
}

/// Enables the player equaliser from presets.
#[derive(CommandModel, CreateCommand)]
#[command(name = "preset")]
pub struct Preset {
    /// Which preset to use?
    preset: EqualiserPreset,
}

impl crate::command::model::BotGuildSlashCommand for Preset {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let preset_name = self.preset.value();
        let update = Some(SetEqualiser::from(self.preset));

        player.update_filter(update).await?;
        ctx.out(format!(
            "🎛️🟢 Enabled player equaliser (Preset: **`{preset_name}`**).",
        ))
        .await?;
        Ok(())
    }
}
