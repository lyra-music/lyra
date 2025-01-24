mod clear;
mod edit;
mod mode;
mod view;

use bitflags::bitflags;
use const_str::concat as const_str_concat;
use itertools::Itertools;
use lyra_ext::{logical_bind::LogicalBind, num::u64_to_i64_truncating};
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
use crate::error::command::check::AccessCalculatorBuildError;
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
        let guild_id = u64_to_i64_truncating(guild_id.get());
        Self {
            set: JoinSet::new(),
            db,
            guild_id,
        }
    }

    fn query(mut self, category: &AccessCategoryFlag, id: i64) -> Self {
        let column = category.ident();
        let db = self.db.clone();
        self.set.spawn(async move {
            let in_access_controls = sqlx::query_as::<_, (Option<bool>,)>(&format!(
                "SELECT EXISTS (SELECT 1 FROM {column} WHERE guild = $1 AND id = $2)"
            ))
            .bind(self.guild_id)
            .bind(id)
            .fetch_one(&db)
            .await?
            .0;

            // SAFETY: `SELECT EXISTS ...` is always non-null
            let in_access_controls = unsafe { in_access_controls.unwrap_unchecked() };

            let (access_mode,) = sqlx::query_as::<_, (Option<bool>,)>(&format!(
                "SELECT {column} FROM guild_configs WHERE id = $1"
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

    pub fn roles<'a>(mut self, role_ids: impl Iterator<Item = &'a Id<RoleMarker>>) -> Self {
        let column = AccessCategoryFlag::Roles.ident();
        let db = self.db.clone();
        let where_clause = role_ids.map(|id| format!("id = {id}")).join(" OR ");
        self.set.spawn(async move {
            let (Some(in_access_controls),) = sqlx::query_as::<_, (Option<bool>,)>(&format!(
                "SELECT EXISTS (SELECT 1 FROM {column} WHERE guild = $1 AND ({}))",
                where_clause.or("true")
            ))
            .bind(self.guild_id)
            .fetch_one(&db)
            .await?
            else {
                panic!("`exists` is `NULL`")
            };

            let access_mode = sqlx::query!(
                "SELECT rol_access FROM guild_configs WHERE id = $1",
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

    pub fn user(self, user_id: Id<UserMarker>) -> Self {
        let id = u64_to_i64_truncating(user_id.get());
        self.query(&AccessCategoryFlag::Users, id)
    }

    pub fn thread(self, thread_id: Id<ChannelMarker>) -> Self {
        let id = u64_to_i64_truncating(thread_id.get());
        self.query(&AccessCategoryFlag::Threads, id)
    }

    pub fn text_channel(self, text_channel_id: Id<ChannelMarker>) -> Self {
        let id = u64_to_i64_truncating(text_channel_id.get());
        self.query(&AccessCategoryFlag::TextChannels, id)
    }

    pub fn voice_channel(self, voice_channel_id: Id<ChannelMarker>) -> Self {
        let id = u64_to_i64_truncating(voice_channel_id.get());
        self.query(&AccessCategoryFlag::VoiceChannels, id)
    }

    pub fn category_channel(self, category_channel_id: Id<ChannelMarker>) -> Self {
        let id = u64_to_i64_truncating(category_channel_id.get());
        self.query(&AccessCategoryFlag::CategoryChannels, id)
    }

    pub async fn build(mut self) -> Result<Calculator, AccessCalculatorBuildError> {
        let mut pairs = Vec::with_capacity(6);
        while let Some(res) = self.set.join_next().await {
            pairs.push(res??);
        }

        Ok(Calculator { pairs })
    }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Hash)]
enum AccessCategoryFlag {
    Users = 0b0000_0001,
    Roles = 0b0000_0010,
    Threads = 0b0000_0100,
    TextChannels = 0b0000_1000,
    VoiceChannels = 0b0001_0000,
    CategoryChannels = 0b0010_0000,
}

impl From<AccessCategoryFlag> for AccessCategoryFlags {
    fn from(value: AccessCategoryFlag) -> Self {
        Self::from_bits_retain(value as u8)
    }
}

impl TryFrom<AccessCategoryFlags> for AccessCategoryFlag {
    type Error = AccessCategoryFlags;

    fn try_from(value: AccessCategoryFlags) -> Result<Self, Self::Error> {
        if value.iter().count() != 1 {
            return Err(value);
        }

        // SAFETY: `value` is guruanteed to only have one flag,
        //         so this transmute is safe
        Ok(unsafe { std::mem::transmute::<u8, Self>(value.bits()) })
    }
}

impl AccessCategoryFlag {
    const fn ident(&self) -> &'static str {
        const POSTFIX: &str = "_access";
        macro_rules! concat_postfix {
            ($postfix: expr) => {
                const_str_concat!($postfix, POSTFIX)
            };
        }

        match self {
            Self::Users => concat_postfix!("usr"),
            Self::Roles => concat_postfix!("rol"),
            Self::Threads => concat_postfix!("xch"),
            Self::TextChannels => concat_postfix!("tch"),
            Self::VoiceChannels => concat_postfix!("vch"),
            Self::CategoryChannels => concat_postfix!("cch"),
        }
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

impl From<AccessCategory> for AccessCategoryFlags {
    fn from(category: AccessCategory) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self::from_bits_retain(category.value().unsigned_abs() as u8)
    }
}

impl AccessCategoryFlags {
    pub fn iter_each(&self) -> impl Iterator<Item = AccessCategoryFlag> {
        self.iter().flat_map(Self::try_into)
    }

    pub fn iter_as_columns(&self) -> impl Iterator<Item = &'static str> {
        self.iter_each().map(|f| f.ident())
    }
}

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "access", desc = ".")]
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
