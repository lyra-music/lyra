use anyhow::Result;
use async_trait::async_trait;
use itertools::Itertools;
use sqlx::{postgres::PgQueryResult, Pool, Postgres};
use tokio::task::JoinSet;
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption, ResolvedMentionable,
};
use twilight_model::{
    application::interaction::application_command::InteractionChannel,
    channel::ChannelType,
    id::{
        marker::{ChannelMarker, GenericMarker},
        Id,
    },
};

use super::AccessCategoryFlags;
use crate::bot::{
    commands::{
        macros::{dub, hid, out},
        models::{App, LyraCommand},
        Context,
    },
    lib::consts::texts::NO_ROWS_AFFECTED_MESSAGE,
};

type SqlxResultJoinSet = JoinSet<Result<PgQueryResult, sqlx::Error>>;

trait AccessCategoryMarker {}

impl AccessCategoryMarker for GenericMarker {}
impl AccessCategoryMarker for ChannelMarker {}

fn add_access<T: AccessCategoryMarker>(
    set: &mut SqlxResultJoinSet,
    db: Pool<Postgres>,
    cat: AccessCategoryFlags,
    g: i64,
    ids: impl IntoIterator<Item = Id<T>>,
) {
    let column = cat
        .iter_names_as_column()
        .next()
        .expect("flags must not be empty");
    let values_clause = ids.into_iter().map(|id| format!("($1,{})", id)).join(",");

    set.spawn(async move {
        sqlx::query(&format!(
            "--sql
            INSERT INTO {0}
                SELECT ch_new.guild, ch_new.id
                FROM (VALUES {1}) AS ch_new (guild, id)
            WHERE NOT EXISTS
                (SELECT 1
                    FROM {0} AS ch
                    WHERE ch.guild = ch_new.guild AND ch.id = ch_new.id
                );
            ",
            column, values_clause
        ))
        .bind(g)
        .execute(&db)
        .await
    });
}

fn remove_access<T: AccessCategoryMarker>(
    set: &mut SqlxResultJoinSet,
    db: Pool<Postgres>,
    cat: AccessCategoryFlags,
    g: i64,
    ids: impl IntoIterator<Item = Id<T>>,
) {
    let column = cat
        .iter_names_as_column()
        .next()
        .expect("flags must not be empty");
    let where_clause = ids.into_iter().map(|c| format!("id = {}", c)).join(" OR ");

    set.spawn(async move {
        sqlx::query(&format!(
            "--sql
            DELETE FROM {0}
            WHERE guild = $1 AND ({1});
            ",
            column, where_clause,
        ))
        .bind(g)
        .execute(&db)
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

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "edit-user-or-role",
    desc = "Edits the currently configured access controls for users or roles"
)]

pub struct EditMemberRole {
    #[command(desc = "Do what?")]
    action: EditAction,
    #[command(desc = "... for whom/which role?")]
    member_or_role: ResolvedMentionable,
    #[command(desc = "... for whom/which role? (2)")]
    member_or_role_2: Option<ResolvedMentionable>,
    #[command(desc = "... for whom/which role? (3)")]
    member_or_role_3: Option<ResolvedMentionable>,
    #[command(desc = "... for whom/which role? (4)")]
    member_or_role_4: Option<ResolvedMentionable>,
    #[command(desc = "... for whom/which role? (5)")]
    member_or_role_5: Option<ResolvedMentionable>,
}

#[async_trait]
impl LyraCommand for EditMemberRole {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        // FIXME: Update this when twilight-interactions support user/role differentiation 1st-party
        let inputted_mentionables = [
            Some(self.member_or_role),
            self.member_or_role_2,
            self.member_or_role_3,
            self.member_or_role_4,
            self.member_or_role_5,
        ]
        .into_iter()
        .flatten()
        .unique_by(|m| m.id())
        .collect::<Vec<_>>();

        let members = (
            AccessCategoryFlags::USERS,
            inputted_mentionables
                .iter()
                .filter_map(|m| matches!(m, ResolvedMentionable::User(_)).then(|| m.id()))
                .collect::<Vec<_>>(),
        );
        let roles = (
            AccessCategoryFlags::ROLES,
            inputted_mentionables
                .iter()
                .filter_map(|m| matches!(m, ResolvedMentionable::Role(_)).then(|| m.id()))
                .collect::<Vec<_>>(),
        );

        let mut set = JoinSet::new();

        let categorized_mentionables = [members, roles]
            .into_iter()
            .filter(|(_, mentionables)| !mentionables.is_empty());

        let db = ctx.db();
        let g = ctx.guild_id_unchecked().get() as i64;
        match self.action {
            EditAction::Add => categorized_mentionables.for_each(|(cat, mentionables)| {
                add_access(&mut set, db.clone(), cat, g, mentionables)
            }),
            EditAction::Remove => categorized_mentionables.for_each(|(cat, mentionables)| {
                remove_access(&mut set, db.clone(), cat, g, mentionables)
            }),
        }

        let mut rows_affected = 0;
        while let Some(res) = set.join_next().await {
            let res = res??;
            rows_affected += res.rows_affected();
        }

        if rows_affected == 0 {
            dub!(NO_ROWS_AFFECTED_MESSAGE, ctx);
        }

        let ignored_changes = inputted_mentionables.len() as u64 - rows_affected;
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

        out!(
            format!(
                "🔐{} {} **`{}`** member(s) or role(s) {} to the guild's access controls{}",
                self.action.as_operator_icon(),
                self.action.as_verb_past(),
                rows_affected,
                self.action.as_associated_preposition(),
                ignored_changes_notice
            ),
            ctx
        );
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "edit-channel",
    desc = "Edits the currently configured access controls for thread, text, voice or category channels"
)]
pub struct EditChannel {
    #[command(desc = "Do what?")]
    action: EditAction,
    #[command(desc = "... for which channel?")]
    channel: InteractionChannel,
    #[command(desc = "... for which channel? (2)")]
    channel_2: Option<InteractionChannel>,
    #[command(desc = "... for which channel? (3)")]
    channel_3: Option<InteractionChannel>,
    #[command(desc = "... for which channel? (4)")]
    channel_4: Option<InteractionChannel>,
    #[command(desc = "... for which channel? (5)")]
    channel_5: Option<InteractionChannel>,
}

#[async_trait]
impl LyraCommand for EditChannel {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        let inputted_channels = [
            Some(self.channel),
            self.channel_2,
            self.channel_3,
            self.channel_4,
            self.channel_5,
        ]
        .into_iter()
        .flatten()
        .unique_by(|c| c.id)
        .collect::<Vec<_>>();

        let threads = (
            AccessCategoryFlags::THREADS,
            inputted_channels
                .iter()
                .filter_map(|c| {
                    matches!(
                        c.kind,
                        ChannelType::PublicThread
                            | ChannelType::PrivateThread
                            | ChannelType::AnnouncementThread
                    )
                    .then(|| c.id)
                })
                .collect::<Vec<_>>(),
        );
        let text_channels = (
            AccessCategoryFlags::TEXT_CHANNELS,
            inputted_channels
                .iter()
                .filter_map(|c| matches!(c.kind, ChannelType::GuildText).then(|| c.id))
                .collect::<Vec<_>>(),
        );
        let voice_channels = (
            AccessCategoryFlags::VOICE_CHANNELS,
            inputted_channels
                .iter()
                .filter_map(|c| {
                    matches!(
                        c.kind,
                        ChannelType::GuildVoice | ChannelType::GuildStageVoice
                    )
                    .then(|| c.id)
                })
                .collect::<Vec<_>>(),
        );
        let category_channels = (
            AccessCategoryFlags::CATEGORY_CHANNELS,
            inputted_channels
                .iter()
                .filter_map(|c| matches!(c.kind, ChannelType::GuildCategory).then(|| c.id))
                .collect::<Vec<_>>(),
        );

        let mut set = JoinSet::new();

        let categorized_channels = [threads, text_channels, voice_channels, category_channels]
            .into_iter()
            .filter(|(_, channels)| !channels.is_empty());

        let db = ctx.db();
        let g = ctx.guild_id_unchecked().get() as i64;
        match self.action {
            EditAction::Add => categorized_channels
                .for_each(|(cat, channels)| add_access(&mut set, db.clone(), cat, g, channels)),
            EditAction::Remove => categorized_channels
                .for_each(|(cat, channels)| remove_access(&mut set, db.clone(), cat, g, channels)),
        }

        let mut rows_affected = 0;
        while let Some(res) = set.join_next().await {
            let res = res??;
            rows_affected += res.rows_affected();
        }

        if rows_affected == 0 {
            dub!(NO_ROWS_AFFECTED_MESSAGE, ctx);
        }

        let ignored_changes = inputted_channels.len() as u64 - rows_affected;
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

        out!(
            format!(
                "🔐{} {} **`{}`** channel(s) {} to the guild's access controls{}",
                self.action.as_operator_icon(),
                self.action.as_verb_past(),
                rows_affected,
                self.action.as_associated_preposition(),
                ignored_changes_notice
            ),
            ctx
        );
    }
}
