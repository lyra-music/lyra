use lyra_ext::{num::u64_to_i64_truncating, pretty::flags_display::FlagsDisplay};
use tokio::task::JoinSet;
use twilight_interactions::command::{CommandModel, CreateCommand};

use super::AccessCategory;
use crate::{
    command::{
        check,
        macros::{note, out, sus},
        model::BotSlashCommand,
        require,
        util::prompt_for_confirmation,
        SlashCtx,
    },
    component::config::access::AccessCategoryFlags,
    core::r#const::text::NO_ROWS_AFFECTED_MESSAGE,
    error::CommandResult,
    gateway::GuildIdAware,
};

/// Clears all currently configured access controls for channels, roles or members
#[derive(CommandModel, CreateCommand)]
#[command(name = "clear")]
pub struct Clear {
    /// Which category(s)?
    category: AccessCategory,
}

impl BotSlashCommand for Clear {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let ctx = require::guild(ctx)?;
        check::user_is_access_manager(&ctx)?;

        let category_flags = AccessCategoryFlags::from(self.category);

        let mut set = JoinSet::new();
        category_flags.iter_as_columns().for_each(|c| {
            let db = ctx.db().clone();
            let g = u64_to_i64_truncating(ctx.guild_id().get());

            set.spawn(async move {
                sqlx::query(&format!("DELETE FROM {c} WHERE guild = $1;"))
                    .bind(g)
                    .execute(&db)
                    .await
            });
        });

        let (mut ctx, confirmed) = prompt_for_confirmation(ctx).await?;
        if !confirmed {
            note!("Cancelled executing this command", ctx);
        }

        let mut rows_affected = 0;
        while let Some(res) = set.join_next().await {
            let res = res??;
            rows_affected += res.rows_affected();
        }

        if rows_affected == 0 {
            sus!(NO_ROWS_AFFECTED_MESSAGE, ctx);
        }

        out!(
            format!(
                "🔐🧹 Cleared all access controls for **{}**.",
                category_flags.pretty_display_code()
            ),
            ctx
        );
    }
}
