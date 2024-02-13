use itertools::Itertools;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};

use super::AccessCategory;
use crate::bot::{
    command::{
        check,
        macros::{hid, out, sus},
        model::{BotSlashCommand, SlashCommand},
        Ctx,
    },
    component::config::access::AccessCategoryFlags,
    core::r#const::text::NO_ROWS_AFFECTED_MESSAGE,
    error::command::Result as CommandResult,
    ext::util::FlagsPrettify,
    gateway::ExpectedGuildIdAware,
};

pub(super) trait AccessModePrettify {
    fn into_mode_icon(self) -> String;
    fn into_verb(self) -> String;
}

impl AccessModePrettify for Option<bool> {
    fn into_mode_icon(self) -> String {
        match self {
            Some(true) => "🟩",
            Some(false) => "🟥",
            None => "⬛",
        }
        .into()
    }
    fn into_verb(self) -> String {
        match self {
            Some(true) => "Allow",
            Some(false) => "Deny",
            None => "Unset",
        }
        .into()
    }
}

#[derive(CommandOption, CreateOption)]
enum AccessMode {
    #[option(name = "Unset", value = 0b00)]
    Unset,
    #[option(name = "Allow", value = 0b01)]
    Allow,
    #[option(name = "Deny", value = 0b10)]
    Deny,
}

impl From<AccessMode> for Option<bool> {
    fn from(val: AccessMode) -> Self {
        match val {
            AccessMode::Unset => None,
            AccessMode::Allow => Some(true),
            AccessMode::Deny => Some(false),
        }
    }
}

/// Sets the access mode for channels, roles or members
#[derive(CommandModel, CreateCommand)]
#[command(name = "mode")]
pub struct Mode {
    /// Set the access mode to which?
    mode: AccessMode,
    /// ...for which category(s)?
    category: AccessCategory,
}

impl BotSlashCommand for Mode {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        check::user_is_access_manager(&ctx)?;

        let access_mode = <Option<bool>>::from(self.mode);
        let sql_access_mode = access_mode.map_or_else(|| "null".into(), |b| b.to_string());

        let category_flags = AccessCategoryFlags::from(self.category);
        let set_statements = category_flags
            .iter_names_as_column()
            .map(|c| format!("{c} = $2"))
            .join(",");
        let where_clause = category_flags
            .iter_names_as_column()
            .map(|c| format!("{c} IS NOT {sql_access_mode}"))
            .join(" OR ");

        let res = sqlx::query(&format!(
            "--sql
            UPDATE guild_configs SET {set_statements} WHERE id = $1 AND ({where_clause});
            "
        ))
        .bind(ctx.guild_id().get() as i64)
        .bind(access_mode)
        .execute(ctx.db())
        .await?;

        if res.rows_affected() == 0 {
            sus!(NO_ROWS_AFFECTED_MESSAGE, ctx);
        }

        let set_unset = access_mode.map_or("Unset", |_| "Set");
        let set_to = access_mode.map_or(String::new(), |m| {
            format!(" to **{}**", Some(m).into_verb())
        });

        out!(
            format!(
                "🔐{} {set_unset} the access mode for **{}**{set_to}.",
                access_mode.into_mode_icon(),
                category_flags.prettify_code(),
            ),
            ctx
        );
    }
}
