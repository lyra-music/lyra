mod autocomplete;
mod command_data;
mod menu;
mod message;
mod modal;

use std::{marker::PhantomData, sync::Arc};

use twilight_cache_inmemory::{
    model::{CachedMember, CachedVoiceState},
    InMemoryCache, Reference,
};
use twilight_gateway::{Latency, MessageSender};
use twilight_http::Client as HttpClient;
use twilight_model::{
    channel::Channel,
    gateway::payload::incoming::InteractionCreate,
    guild::{PartialMember, Permissions},
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
    user::User,
};

use crate::bot::{
    core::model::{
        AuthorPermissionsAware, BotState, BotStateAware, CacheAware, HttpAware,
        InteractionInterface, OwnedBotState, OwnedBotStateAware,
    },
    error::{command::RespondError, core::DeserializeBodyFromHttpError},
    gateway::{ExpectedGuildIdAware, GuildIdAware, SenderAware},
    lavalink::{
        ExpectedPlayerAware, ExpectedPlayerDataAware, Lavalink, LavalinkAware, PlayerAware,
        PlayerDataAware,
    },
};

use super::PartialInteractionData;

use self::modal::ModalMarker;
pub use self::{
    autocomplete::AutocompleteCtx,
    command_data::CommandDataAware,
    menu::{MessageCtx, UserCtx},
    message::RespondViaMessage,
    modal::{ModalCtx, RespondViaModal},
};

type RespondResult<T> = Result<T, RespondError>;
type UnitRespondResult = RespondResult<()>;
type CurrentVoiceStateResult<'a> =
    Option<Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedVoiceState>>;
type CachedBotMember<'a> = Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedMember>;

pub trait CtxKind {}

pub trait AppCtxKind {}

pub struct SlashAppMarker;
impl AppCtxKind for SlashAppMarker {}

pub struct AppCtxMarker<T: AppCtxKind>(PhantomData<fn(T) -> T>);
impl<T: AppCtxKind> CtxKind for AppCtxMarker<T> {}

pub type SlashMarker = AppCtxMarker<SlashAppMarker>;
pub type SlashCtx = Ctx<SlashMarker>;

pub struct ComponentMarker;
impl CtxKind for ComponentMarker {}

pub type ComponentCtx = Ctx<ComponentMarker>;

pub struct Ctx<T: CtxKind> {
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
    data: Option<PartialInteractionData>,
    acknowledged: bool,
    kind: PhantomData<fn(T) -> T>,
}

impl<T: CtxKind> Ctx<T> {
    pub fn into_modal_interaction(self, inner: Box<InteractionCreate>) -> ModalCtx {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            data: None,
            acknowledged: false,
            kind: PhantomData::<fn(ModalMarker) -> ModalMarker>,
        }
    }

    pub const fn latency(&self) -> &Latency {
        &self.latency
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    pub fn db(&self) -> &sqlx::Pool<sqlx::Postgres> {
        self.bot.db()
    }

    pub fn bot_member(&self) -> CachedBotMember {
        self.cache()
            .member(self.guild_id(), self.bot().user_id())
            .expect("bot's member should be in cache")
    }

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn channel(&self) -> &Channel {
        self.inner
            .channel
            .as_ref()
            .expect("interaction type is not ping")
    }

    pub fn author(&self) -> &User {
        self.inner.author().expect("interaction type is not ping")
    }

    pub fn member(&self) -> &PartialMember {
        self.inner
            .member
            .as_ref()
            .expect("interaction invoked in a guild")
    }

    #[inline]
    pub fn author_id(&self) -> Id<UserMarker> {
        self.author().id
    }

    pub fn interaction(&self) -> &InteractionCreate {
        self.inner.as_ref()
    }

    pub fn interaction_token(&self) -> &str {
        &self.inner.token
    }

    pub async fn interface(&self) -> Result<InteractionInterface, DeserializeBodyFromHttpError> {
        Ok(self.bot.interaction().await?.interfaces(&self.inner))
    }

    pub fn bot_permissions(&self) -> Permissions {
        self.inner
            .app_permissions
            .expect("interaction invoked in a guild")
    }

    pub fn bot_permissions_for(&self, channel_id: Id<ChannelMarker>) -> Permissions {
        if channel_id == self.channel_id() {
            return self.bot_permissions();
        }

        let guild_id = self.guild_id();
        let user_id = self.bot().user_id();
        let everyone_role = self
            .cache()
            .role(guild_id.cast())
            .expect("@everyone role should be in cache");
        let member_roles = self
            .bot_member()
            .roles()
            .iter()
            .filter(|&&r| r != everyone_role.id)
            .filter_map(|&r| {
                let role = self.cache().role(r)?;

                Some((r, role.permissions))
            })
            .collect::<Box<_>>();

        let channel = self
            .cache()
            .channel(channel_id)
            .expect("channel should be in cache");
        let channel_overwrites = channel
            .permission_overwrites
            .as_ref()
            .expect("channel is in a guild");

        twilight_util::permission_calculator::PermissionCalculator::new(
            guild_id,
            user_id,
            everyone_role.permissions,
            &member_roles,
        )
        .in_channel(channel.kind, channel_overwrites)
    }

    pub fn current_voice_state(&self) -> CurrentVoiceStateResult {
        let user = self.bot().user_id();
        self.cache().voice_state(user, self.get_guild_id()?)
    }
}

impl<T: CtxKind> BotStateAware for Ctx<T> {
    fn bot(&self) -> &BotState {
        &self.bot
    }
}

impl<T: CtxKind> OwnedBotStateAware for Ctx<T> {
    fn bot_owned(&self) -> Arc<BotState> {
        self.bot.clone()
    }
}

impl<T: CtxKind> SenderAware for Ctx<T> {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl<T: CtxKind> CacheAware for Ctx<T> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl<T: CtxKind> HttpAware for Ctx<T> {
    fn http(&self) -> &HttpClient {
        self.bot.http()
    }
}

impl<T: CtxKind> LavalinkAware for Ctx<T> {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }
}

impl<T: CtxKind> PlayerDataAware for Ctx<T> {}
impl<T: CtxKind> ExpectedPlayerDataAware for Ctx<T> {}
impl<T: CtxKind> PlayerAware for Ctx<T> {}
impl<T: CtxKind> ExpectedPlayerAware for Ctx<T> {}

impl<T: CtxKind> GuildIdAware for Ctx<T> {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>> {
        self.inner.guild_id
    }
}

impl<T: CtxKind> ExpectedGuildIdAware for Ctx<T> {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.get_guild_id().expect("interaction invoked in a guild")
    }
}

impl<T: CtxKind> AuthorPermissionsAware for Ctx<T> {
    fn author_permissions(&self) -> Permissions {
        self.inner
            .member
            .as_ref()
            .expect("interaction invoked in a guild")
            .permissions
            .expect("member from an interaction")
    }
}
