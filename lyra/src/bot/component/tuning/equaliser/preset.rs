use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{command::macros::out, component::tuning::equaliser::SetEqualiser};

lyra_proc::read_equaliser_presets_as!(EqualiserPreset);

impl From<EqualiserPreset> for SetEqualiser {
    fn from(value: EqualiserPreset) -> Self {
        let gains = value.gains();
        Self(core::array::from_fn(|i| {
            lavalink_rs::model::player::Equalizer {
                band: i as u8,
                gain: gains[i],
            }
        }))
    }
}

/// Enable the player equaliser from presets
#[derive(CommandModel, CreateCommand)]
#[command(name = "preset")]
pub struct Preset {
    /// Which preset to use?
    preset: EqualiserPreset,
}

impl crate::bot::command::model::BotSlashCommand for Preset {
    async fn run(self, mut ctx: crate::bot::command::SlashCtx) -> crate::bot::error::CommandResult {
        super::super::common_checks(&ctx)?;

        let preset_name = self.preset.value();
        let update = Some(SetEqualiser::from(self.preset));

        super::super::set_filter(&ctx, update).await?;
        out!(
            format!(
                "🎛️🟢 Enabled player equaliser (Preset: **`{}`**)",
                preset_name
            ),
            ctx
        );
    }
}
