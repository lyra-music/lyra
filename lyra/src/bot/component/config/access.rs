mod clear;
mod edit;
mod mode;
mod view;

use bitflags::bitflags;
use itertools::Itertools;
use sqlx::{Pool, Postgres};
use tokio::task::JoinSet;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
    Id,
};

pub use self::{
    clear::Clear,
    edit::{Channel as EditChannel, MemberRole as EditMemberRole},
    mode::Mode,
    view::View,
};
use crate::bot::{
    command::model::{BotSlashCommand, CommandInfoAware, Ctx},
    component::config::SlashCommand,
    error::command::{check::AccessCalculatorBuildError, Result as CommandResult},
    ext::util::{FlagsPrettify, OptionMap},
};
use lyra_proc::BotCommandGroup;

struct AccessState {
    in_access_controls: bool,
    access_mode: Option<bool>,
}

pub struct Calculator {
    pairs: Vec<AccessState>,
}

impl Calculator {
    pub fn calculate(self) -> bool {
        self.pairs.into_iter().all(
            |AccessState {
                 in_access_controls,
                 access_mode,
             }| {
                access_mode.map_or(true, |access_mode| access_mode == in_access_controls)
            },
        )
    }
}

pub struct CalculatorBuilder {
    set: JoinSet<Result<AccessState, sqlx::Error>>,
    db: Pool<Postgres>,
    guild_id: i64,
}

impl CalculatorBuilder {
    pub fn new(guild_id: Id<GuildMarker>, db: Pool<Postgres>) -> Self {
        let guild_id = guild_id.get() as i64;
        Self {
            set: JoinSet::new(),
            db,
            guild_id,
        }
    }

    fn query(mut self, column: &'static str, id: i64) -> Self {
        let db = self.db.clone();
        self.set.spawn(async move {
            let (Some(in_access_controls),) = sqlx::query_as::<_, (Option<bool>,)>(&format!(
                "--sql
                SELECT EXISTS (SELECT 1 FROM {column} WHERE guild = $1 AND id = $2) 
                "
            ))
            .bind(self.guild_id)
            .bind(id)
            .fetch_one(&db)
            .await?
            else {
                panic!("`exists` must not be `NULL`")
            };

            let (access_mode,) = sqlx::query_as::<_, (Option<bool>,)>(&format!(
                "--sql
                SELECT {column} FROM guild_configs WHERE id = $1
                "
            ))
            .bind(self.guild_id)
            .fetch_one(&db)
            .await?;

            Ok(AccessState {
                in_access_controls,
                access_mode,
            })
        });
        self
    }

    pub fn user(self, user_id: Id<UserMarker>) -> Self {
        let id = user_id.get() as i64;
        self.query("usr_access", id)
    }

    pub fn roles<'a>(mut self, role_ids: impl Iterator<Item = &'a Id<RoleMarker>>) -> Self {
        let db = self.db.clone();
        let where_clause = role_ids.map(|id| format!("id = {id}")).join(" OR ");
        self.set.spawn(async move {
            let (Some(in_access_controls),) = sqlx::query_as::<_, (Option<bool>,)>(&format!(
                "--sql
                SELECT EXISTS (SELECT 1 FROM rol_access WHERE guild = $1 AND ({})) 
                ",
                where_clause.or("true")
            ))
            .bind(self.guild_id)
            .fetch_one(&db)
            .await?
            else {
                panic!("`exists` must not be `NULL`")
            };

            let access_mode = sqlx::query!(
                r"--sql
                SELECT rol_access FROM guild_configs WHERE id = $1
                ",
                self.guild_id
            )
            .fetch_one(&db)
            .await?
            .rol_access;

            Ok(AccessState {
                in_access_controls,
                access_mode,
            })
        });
        self
    }

    pub fn thread(self, thread_id: Id<ChannelMarker>) -> Self {
        let id = thread_id.get() as i64;
        self.query("xch_access", id)
    }

    pub fn text_channel(self, text_channel_id: Id<ChannelMarker>) -> Self {
        let id = text_channel_id.get() as i64;
        self.query("tch_access", id)
    }

    pub fn voice_channel(self, voice_channel_id: Id<ChannelMarker>) -> Self {
        let id = voice_channel_id.get() as i64;
        self.query("vch_access", id)
    }

    pub fn category_channel(self, category_channel_id: Id<ChannelMarker>) -> Self {
        let id = category_channel_id.get() as i64;
        self.query("cch_access", id)
    }

    pub async fn build(mut self) -> Result<Calculator, AccessCalculatorBuildError> {
        let mut pairs = Vec::with_capacity(6);
        while let Some(res) = self.set.join_next().await {
            pairs.push(res??);
        }

        Ok(Calculator { pairs })
    }
}

bitflags! {
    struct AccessCategoryFlags: u8 {
        const USERS = 0b0000_0001;
        const ROLES = 0b0000_0010;
        const THREADS = 0b0000_0100;
        const TEXT_CHANNELS = 0b0000_1000;
        const VOICE_CHANNELS = 0b0001_0000;
        const CATEGORY_CHANNELS = 0b0010_0000;

        const MENTIONABLES = Self::USERS.bits() | Self::ROLES.bits();
        const ALL_CHANNELS = Self::THREADS.bits()
            | Self::TEXT_CHANNELS.bits()
            | Self::VOICE_CHANNELS.bits()
            | Self::CATEGORY_CHANNELS.bits();

        const ALL = Self::MENTIONABLES.bits()
            | Self::ALL_CHANNELS.bits();
    }
}

impl FlagsPrettify for AccessCategoryFlags {}

impl From<AccessCategory> for AccessCategoryFlags {
    fn from(category: AccessCategory) -> Self {
        Self::from_bits_truncate(category.value() as u8)
    }
}

impl AccessCategoryFlags {
    pub fn iter_names_as_column(&self) -> impl Iterator<Item = String> {
        self.iter_names()
            .map(|(n, _)| match n {
                "USERS" => "usr",
                "ROLES" => "rol",
                "THREADS" => "xch",
                "TEXT_CHANNELS" => "tch",
                "VOICE_CHANNELS" => "vch",
                "CATEGORY_CHANNELS" => "cch",
                _ => unreachable!(),
            })
            .map(|n| format!("{n}_access"))
    }
}

/// -
#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "access")]
pub enum Access {
    #[command(name = "view")]
    View(View),
    #[command(name = "edit-channel")]
    EditChannel(Box<EditChannel>),
    #[command(name = "edit-user-or-role")]
    EditMemberRole(Box<EditMemberRole>),
    #[command(name = "mode")]
    Mode(Mode),
    #[command(name = "clear")]
    Clear(Clear),
}

#[derive(CommandOption, CreateOption)]
enum AccessCategory {
    #[option(name = "Users", value = 0b0000_0001)]
    Users,
    #[option(name = "Roles", value = 0b0000_0010)]
    Roles,
    #[option(name = "Threads", value = 0b0000_0100)]
    Threads,
    #[option(name = "Text Channels", value = 0b0000_1000)]
    TextChannels,
    #[option(name = "Voice Channels", value = 0b0001_0000)]
    VoiceChannels,
    #[option(name = "Catgeory Channels", value = 0b0010_0000)]
    CategoryChannels,
    #[option(name = "Users & Roles", value = 0b0000_0011)]
    Mentionables,
    #[option(name = "Text, Voice & Category Channels", value = 0b0011_1100)]
    AllChannels,
    #[option(name = "All Categories", value = 0b0011_1111)]
    All,
}
