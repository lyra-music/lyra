use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::macros::out,
    component::tuning::{common_checks, set_filter},
};

/// Disable the player equaliser
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl crate::bot::command::model::BotSlashCommand for Off {
    async fn run(self, mut ctx: crate::bot::command::SlashCtx) -> crate::bot::error::CommandResult {
        common_checks(&ctx)?;

        set_filter(&ctx, None::<super::SetEqualiser>).await?;
        out!("🎛️🔴 Disabled equaliser", ctx);
    }
}
