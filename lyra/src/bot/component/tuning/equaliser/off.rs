use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{macros::out, require},
    component::tuning::{check_user_is_dj_and_require_unsuppressed_player, UpdateFilter},
};

/// Disable the player equaliser
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl crate::bot::command::model::BotSlashCommand for Off {
    async fn run(self, ctx: crate::bot::command::SlashCtx) -> crate::bot::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        player.update_filter(None::<super::SetEqualiser>).await?;
        out!("🎛️🔴 Disabled equaliser", ctx);
    }
}
