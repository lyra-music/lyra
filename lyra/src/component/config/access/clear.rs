use lyra_ext::pretty::flags_display::FlagsDisplay;
use tokio::task::JoinSet;
use twilight_interactions::command::{CommandModel, CreateCommand};

use super::AccessCategory;
use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        util::prompt_for_confirmation,
    },
    component::config::access::AccessCategoryFlags,
    core::{
        konst::text::NO_ROWS_AFFECTED_MESSAGE,
        model::{DatabaseAware, response::initial::message::create::RespondWithMessage},
    },
    error::CommandResult,
    gateway::GuildIdAware,
};

/// Clears all currently configured access controls for channels, roles or members.
#[derive(CommandModel, CreateCommand)]
#[command(name = "clear")]
pub struct Clear {
    /// Which category(s)?
    category: AccessCategory,
}

impl BotGuildSlashCommand for Clear {
    async fn run(self, ctx: GuildSlashCmdCtx) -> CommandResult {
        check::user_is_access_manager(&ctx)?;

        let category_flags = AccessCategoryFlags::from(self.category);

        let mut set = JoinSet::new();
        category_flags.iter_as_columns().for_each(|c| {
            let db = ctx.db().clone();
            let g = ctx.guild_id().get().cast_signed();

            set.spawn(async move {
                sqlx::query(&format!("DELETE FROM {c} WHERE guild = $1;"))
                    .bind(g)
                    .execute(&db)
                    .await
            });
        });

        let (mut ctx, confirmed) = prompt_for_confirmation(ctx).await?;
        if !confirmed {
            ctx.note("Cancelled executing this command.").await?;
            return Ok(());
        }

        let mut rows_affected = 0;
        while let Some(res) = set.join_next().await {
            let res = res??;
            rows_affected += res.rows_affected();
        }

        if rows_affected == 0 {
            ctx.susp(NO_ROWS_AFFECTED_MESSAGE).await?;
            return Ok(());
        }

        ctx.out(format!(
            "🔐🧹 Cleared all access controls for **{}**.",
            category_flags.pretty_display_code()
        ))
        .await?;
        Ok(())
    }
}
