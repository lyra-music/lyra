use anyhow::Result;
use async_trait::async_trait;
use itertools::Itertools;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};

use super::AccessCategory;
use crate::bot::{
    commands::{
        macros::{dub, hid, out},
        models::{App, LyraCommand},
        Context,
    },
    ext::utils::FlagsPrettify,
    lib::consts::texts::NO_ROWS_AFFECTED_MESSAGE,
    modules::config::access::AccessCategoryFlags,
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

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "mode",
    desc = "Sets the access mode for channels, roles or members"
)]
pub struct Mode {
    #[command(desc = "Set the access mode to which?")]
    mode: AccessMode,
    #[command(desc = "...for which category(s)?")]
    category: AccessCategory,
}

#[async_trait]
impl LyraCommand for Mode {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        let access_mode: Option<bool> = self.mode.into();
        let sql_access_mode = match access_mode {
            Some(b) => b.to_string(),
            None => "null".into(),
        };

        let category_flags: AccessCategoryFlags = self.category.into();
        let set_statements = category_flags
            .iter_names_as_column()
            .map(|c| format!("{} = $2", c))
            .join(",");
        let where_clause = category_flags
            .iter_names_as_column()
            .map(|c| format!("{} IS NOT {}", c, sql_access_mode))
            .join(" OR ");

        let res = sqlx::query(&format!(
            "--sql
            UPDATE guild_configs SET {set_statements} WHERE id = $1 AND ({where_clause});
            "
        ))
        .bind(ctx.guild_id_unchecked().get() as i64)
        .bind(access_mode)
        .execute(ctx.db())
        .await?;

        if res.rows_affected() == 0 {
            dub!(NO_ROWS_AFFECTED_MESSAGE, ctx);
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
