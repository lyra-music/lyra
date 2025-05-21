use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use lyra_ext::num::u64_to_i64_truncating;
use sqlx::{Pool, Postgres, postgres::PgQueryResult};
use tokio::task::JoinSet;
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption, ResolvedMentionable,
};
use twilight_model::{
    application::interaction::InteractionChannel,
    channel::ChannelType,
    id::{
        Id,
        marker::{ChannelMarker, GenericMarker},
    },
};

use super::AccessCategoryFlag;
use crate::{
    command::{SlashCmdCtx, check, model::BotSlashCommand, require},
    core::{
        r#const::text::NO_ROWS_AFFECTED_MESSAGE,
        model::{DatabaseAware, response::initial::message::create::RespondWithMessage},
    },
    error::CommandResult,
    gateway::GuildIdAware,
};

type SqlxResultJoinSet = JoinSet<Result<PgQueryResult, sqlx::Error>>;

trait AccessCategoryMarker {}

impl AccessCategoryMarker for GenericMarker {}
impl AccessCategoryMarker for ChannelMarker {}

fn add_access<T: AccessCategoryMarker>(
    join_set: &mut SqlxResultJoinSet,
    database: Pool<Postgres>,
    category: &AccessCategoryFlag,
    guild_id: i64,
    ids: impl IntoIterator<Item = Id<T>>,
) {
    let column = category.ident();
    let values_clause = ids.into_iter().map(|id| format!("($1,{id})")).join(",");

    join_set.spawn(async move {
        sqlx::query(&format!(
            "INSERT INTO {column}
                SELECT ch_new.guild, ch_new.id
                FROM (VALUES {values_clause}) AS ch_new (guild, id)
            WHERE NOT EXISTS
                (SELECT 1
                    FROM {column} AS ch
                    WHERE ch.guild = ch_new.guild AND ch.id = ch_new.id
                );
            "
        ))
        .bind(guild_id)
        .execute(&database)
        .await
    });
}

fn remove_access<T: AccessCategoryMarker>(
    join_set: &mut SqlxResultJoinSet,
    database: Pool<Postgres>,
    category: &AccessCategoryFlag,
    guild_id: i64,
    ids: impl IntoIterator<Item = Id<T>>,
) {
    let column = category.ident();
    let where_clause = ids.into_iter().map(|c| format!("id = {c}")).join(" OR ");

    join_set.spawn(async move {
        sqlx::query(&format!(
            "DELETE FROM {column}
            WHERE guild = $1 AND ({where_clause});
            ",
        ))
        .bind(guild_id)
        .execute(&database)
        .await
    });
}

#[derive(CommandOption, CreateOption)]
enum EditAction {
    #[option(name = "Add", value = 0b01)]
    Add,
    #[option(name = "Remove", value = 0b10)]
    Remove,
}

trait EditActionPrettify {
    fn as_operator_icon(&self) -> String;
    fn as_verb_past(&self) -> String;
    fn as_associated_preposition(&self) -> String;
    fn as_ignored_reason(&self) -> String;
}

impl EditActionPrettify for EditAction {
    fn as_operator_icon(&self) -> String {
        match self {
            Self::Add => "**`＋`**",
            Self::Remove => "**`－`**",
        }
        .into()
    }

    fn as_verb_past(&self) -> String {
        match self {
            Self::Add => "Added",
            Self::Remove => "Removed",
        }
        .into()
    }

    fn as_associated_preposition(&self) -> String {
        match self {
            Self::Add => "to",
            Self::Remove => "from",
        }
        .into()
    }
    fn as_ignored_reason(&self) -> String {
        match self {
            Self::Add => "already existing",
            Self::Remove => "non-existing",
        }
        .into()
    }
}

/// Edits the currently configured access controls for users or roles.
#[derive(CommandModel, CreateCommand)]
#[command(name = "edit-user-or-role")]
pub struct MemberRole {
    /// Do what?
    action: EditAction,
    /// ... for whom/which role?
    member_or_role: ResolvedMentionable,
    /// ... for whom/which role? (2)
    member_or_role_2: Option<ResolvedMentionable>,
    /// ... for whom/which role? (3)
    member_or_role_3: Option<ResolvedMentionable>,
    /// ... for whom/which role? (4)
    member_or_role_4: Option<ResolvedMentionable>,
    /// ... for whom/which role? (5)
    member_or_role_5: Option<ResolvedMentionable>,
}

impl BotSlashCommand for MemberRole {
    async fn run(self, ctx: SlashCmdCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        check::user_is_access_manager(&ctx)?;

        let input_mentionables: HashMap<_, HashSet<_>> = [
            Some(self.member_or_role),
            self.member_or_role_2,
            self.member_or_role_3,
            self.member_or_role_4,
            self.member_or_role_5,
        ]
        .into_iter()
        .flatten()
        .map(|v| {
            let flag = match v {
                ResolvedMentionable::User(_) => AccessCategoryFlag::Users,
                ResolvedMentionable::Role(_) => AccessCategoryFlag::Roles,
            };
            (flag, v.id())
        })
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k).or_default().insert(v);
            acc
        });

        let input_mentionables_len = input_mentionables.values().fold(0, |acc, v| acc + v.len());

        let database = ctx.db();
        let guild_id = u64_to_i64_truncating(ctx.guild_id().get());
        let mut set = JoinSet::new();
        match self.action {
            EditAction::Add => {
                for (category, mentionables) in input_mentionables {
                    add_access(
                        &mut set,
                        database.clone(),
                        &category,
                        guild_id,
                        mentionables,
                    );
                }
            }
            EditAction::Remove => {
                for (category, mentionables) in input_mentionables {
                    remove_access(
                        &mut set,
                        database.clone(),
                        &category,
                        guild_id,
                        mentionables,
                    );
                }
            }
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

        let ignored_changes = input_mentionables_len as u64 - rows_affected;
        let ignored_changes_notice = match ignored_changes {
            1.. => {
                format!(
                    " `(Ignored {} {} access control(s))`",
                    ignored_changes,
                    self.action.as_ignored_reason()
                )
            }
            _ => String::new(),
        };

        ctx.out(format!(
            "🔐{} {} **`{}`** member(s) or role(s) {} to the guild's access controls{}.",
            self.action.as_operator_icon(),
            self.action.as_verb_past(),
            rows_affected,
            self.action.as_associated_preposition(),
            ignored_changes_notice
        ))
        .await?;
        Ok(())
    }
}

/// Edits the currently configured access controls for thread, text, voice or category channels
#[derive(CommandModel, CreateCommand)]
#[command(name = "edit-channel")]
pub struct Channel {
    /// Do what?
    action: EditAction,
    /// ... for which channel?
    target: InteractionChannel,
    /// ... for which channel? (2)
    target_2: Option<InteractionChannel>,
    /// ... for which channel? (3)
    target_3: Option<InteractionChannel>,
    /// ... for which channel? (4)
    target_4: Option<InteractionChannel>,
    /// ... for which channel? (5)
    target_5: Option<InteractionChannel>,
}

impl BotSlashCommand for Channel {
    async fn run(self, ctx: SlashCmdCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        check::user_is_access_manager(&ctx)?;

        let input_channels: HashMap<_, HashSet<_>> = [
            Some(self.target),
            self.target_2,
            self.target_3,
            self.target_4,
            self.target_5,
        ]
        .into_iter()
        .flatten()
        .map(|v| {
            let flag = match v.kind {
                ChannelType::PublicThread
                | ChannelType::PrivateThread
                | ChannelType::AnnouncementThread => AccessCategoryFlag::Threads,
                ChannelType::GuildVoice | ChannelType::GuildStageVoice => {
                    AccessCategoryFlag::VoiceChannels
                }
                ChannelType::GuildCategory => AccessCategoryFlag::CategoryChannels,
                _ => AccessCategoryFlag::TextChannels,
            };
            (flag, v.id)
        })
        .fold(HashMap::new(), |mut acc, (k, v)| {
            acc.entry(k).or_default().insert(v);
            acc
        });

        let input_channels_len = input_channels.values().fold(0, |acc, v| acc + v.len());

        let database = ctx.db();
        let guild_id = u64_to_i64_truncating(ctx.guild_id().get());
        let mut set = JoinSet::new();
        match self.action {
            EditAction::Add => {
                for (category, channels) in input_channels {
                    add_access(&mut set, database.clone(), &category, guild_id, channels);
                }
            }
            EditAction::Remove => {
                for (category, channels) in input_channels {
                    remove_access(&mut set, database.clone(), &category, guild_id, channels);
                }
            }
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

        let ignored_changes = input_channels_len as u64 - rows_affected;
        let ignored_changes_notice = match ignored_changes {
            1.. => {
                format!(
                    " `(Ignored {} {} access control(s))`",
                    ignored_changes,
                    self.action.as_ignored_reason()
                )
            }
            _ => String::new(),
        };

        ctx.out(format!(
            "🔐{} {} **`{}`** channel(s) {} to the guild's access controls{}",
            self.action.as_operator_icon(),
            self.action.as_verb_past(),
            rows_affected,
            self.action.as_associated_preposition(),
            ignored_changes_notice
        ))
        .await?;
        Ok(())
    }
}
