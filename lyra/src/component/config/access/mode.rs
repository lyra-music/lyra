use std::fmt::Display;

use itertools::Itertools;
use lyra_ext::{num::u64_to_i64_truncating, pretty::flags_display::FlagsDisplay};
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};

use super::AccessCategory;
use crate::{
    command::{
        check,
        macros::{out, sus},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    component::config::access::AccessCategoryFlags,
    core::r#const::text::NO_ROWS_AFFECTED_MESSAGE,
    error::CommandResult,
    gateway::GuildIdAware,
};

enum AccessModeDisplayType {
    Icon,
    Verb,
}

pub(super) struct AccessModeDisplayer {
    inner: Option<bool>,
    kind: AccessModeDisplayType,
}

impl Display for AccessModeDisplayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner;
        let s = match self.kind {
            AccessModeDisplayType::Icon => match inner {
                Some(true) => "🟩",
                Some(false) => "🟥",
                None => "⬛",
            },
            AccessModeDisplayType::Verb => match inner {
                Some(true) => "Allow",
                Some(false) => "Deny",
                None => "Unset",
            },
        };

        f.write_str(s)
    }
}

pub(super) trait AccessModeDisplay {
    fn display_icon(self) -> AccessModeDisplayer;
    fn display_verb(self) -> AccessModeDisplayer;
}

impl AccessModeDisplay for Option<bool> {
    fn display_icon(self) -> AccessModeDisplayer {
        AccessModeDisplayer {
            inner: self,
            kind: AccessModeDisplayType::Icon,
        }
    }

    fn display_verb(self) -> AccessModeDisplayer {
        AccessModeDisplayer {
            inner: self,
            kind: AccessModeDisplayType::Verb,
        }
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
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        check::user_is_access_manager(&ctx)?;

        let access_mode = <Option<bool>>::from(self.mode);
        let sql_access_mode = access_mode.map_or_else(|| String::from("null"), |b| b.to_string());

        let category_flags = AccessCategoryFlags::from(self.category);
        let set_statements = category_flags
            .iter_as_columns()
            .map(|c| format!("{c} = $2"))
            .join(",");
        let where_clause = category_flags
            .iter_as_columns()
            .map(|c| format!("{c} IS NOT {sql_access_mode}"))
            .join(" OR ");

        let res = sqlx::query(&format!(
            "UPDATE guild_configs SET {set_statements} WHERE id = $1 AND ({where_clause});"
        ))
        .bind(u64_to_i64_truncating(ctx.guild_id().get()))
        .bind(access_mode)
        .execute(ctx.db())
        .await?;

        if res.rows_affected() == 0 {
            sus!(NO_ROWS_AFFECTED_MESSAGE, ctx);
        }

        let set_unset = access_mode.map_or("Unset", |_| "Set");
        let set_to = access_mode.map_or(String::new(), |m| {
            format!(" to **{}**", Some(m).display_verb())
        });

        out!(
            format!(
                "🔐{} {set_unset} the access mode for **{}**{set_to}.",
                access_mode.display_icon(),
                category_flags.pretty_display_code(),
            ),
            ctx
        );
    }
}
