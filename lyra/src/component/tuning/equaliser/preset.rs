use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::GuildSlashCmdCtx,
    component::tuning::{
        UpdateFilter, check_user_is_dj_and_require_unsuppressed_player, equaliser::SetEqualiser,
    },
    core::model::response::initial::message::create::RespondWithMessage,
};

lyra_proc::read_equaliser_presets_as!(EqualiserPreset);

impl From<EqualiserPreset> for SetEqualiser {
    fn from(value: EqualiserPreset) -> Self {
        let gains = value.gains();
        Self(core::array::from_fn(|i| {
            lavalink_rs::model::player::Equalizer {
                #[expect(clippy::cast_possible_truncation)]
                band: i as u8,
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
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

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
