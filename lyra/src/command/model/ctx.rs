mod autocomplete;
mod command_data;
mod menu;
mod message;
mod modal;

use std::{marker::PhantomData, sync::Arc};

use twilight_cache_inmemory::{model::CachedMember, InMemoryCache, Reference};
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

use crate::{
    command::{model::NonPingInteraction, require::CachedVoiceStateRef},
    core::model::{
        AuthorIdAware, AuthorPermissionsAware, BotState, BotStateAware, CacheAware, HttpAware,
        InteractionInterface, OwnedBotState, OwnedBotStateAware,
    },
    error::{
        command::RespondError, core::DeserializeBodyFromHttpError, Cache, CacheResult, NotInGuild,
    },
    gateway::{GuildIdAware, OptionallyGuildIdAware, SenderAware},
    lavalink::{Lavalink, LavalinkAware, PlayerAware},
};

use super::PartialInteractionData;

use self::modal::ModalMarker;
pub use self::{
    autocomplete::AutocompleteCtx,
    command_data::CommandDataAware,
    menu::{MessageCtx, UserCtx},
    message::RespondViaMessage,
    modal::{GuildModalCtx, RespondViaModal},
};

type RespondResult<T> = Result<T, RespondError>;
type UnitRespondResult = RespondResult<()>;
type CachedBotMember<'a> = Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedMember>;

pub trait CtxKind {}

pub trait AppCtxKind {}

pub struct SlashAppMarker;
impl AppCtxKind for SlashAppMarker {}

pub struct AppCtxMarker<T: AppCtxKind>(PhantomData<fn(T) -> T>);
impl<T: AppCtxKind> CtxKind for AppCtxMarker<T> {}

pub type SlashMarker = AppCtxMarker<SlashAppMarker>;
pub type SlashCtx = Ctx<SlashMarker>;
pub type GuildSlashCtx = Ctx<SlashMarker, Guild>;

pub struct ComponentMarker;
impl CtxKind for ComponentMarker {}

pub type ComponentCtx = Ctx<ComponentMarker>;

pub trait CtxLocation {}

pub struct Unknown;
impl CtxLocation for Unknown {}

pub struct Guild;
impl CtxLocation for Guild {}
pub type GuildCtx<T> = Ctx<T, Guild>;

pub struct Ctx<Of: CtxKind, In: CtxLocation = Unknown> {
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
    data: Option<PartialInteractionData>,
    acknowledged: bool,
    kind: PhantomData<fn(Of) -> Of>,
    location: PhantomData<fn(In) -> In>,
}

impl<T: CtxKind> TryFrom<Ctx<T>> for Ctx<T, Guild> {
    type Error = NotInGuild;

    fn try_from(value: Ctx<T>) -> Result<Self, Self::Error> {
        value.get_guild_id().ok_or(NotInGuild)?;
        Ok(Self {
            inner: value.inner,
            bot: value.bot,
            latency: value.latency,
            sender: value.sender,
            data: value.data,
            acknowledged: value.acknowledged,
            kind: value.kind,
            location: PhantomData::<fn(Guild) -> Guild>,
        })
    }
}

impl<T: CtxKind, U: CtxLocation> Ctx<T, U> {
    pub fn into_modal_interaction(self, inner: Box<InteractionCreate>) -> Ctx<ModalMarker, U> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            location: self.location,
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

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn channel(&self) -> &Channel {
        // SAFETY: Interaction type is not `Ping`, so `channel` is present.
        unsafe { self.inner.channel_unchecked() }
    }

    pub fn author(&self) -> &User {
        // SAFETY: Interaction type is not `Ping`, so `author()` is present.
        unsafe { self.inner.author_unchecked() }
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

    pub unsafe fn guild_id_unchecked(&self) -> Id<GuildMarker> {
        // SAFETY: this interaction was invoked in a guild,
        //         so `self.inner.guild_id` is present
        unsafe { self.get_guild_id().unwrap_unchecked() }
    }

    pub unsafe fn member_unchecked(&self) -> &PartialMember {
        // SAFETY: this interaction was invoked in a guild,
        //         so `self.inner.member` is present
        unsafe { self.inner.member.as_ref().unwrap_unchecked() }
    }

    pub unsafe fn bot_permissions_unchecked(&self) -> Permissions {
        // SAFETY: this interaction was invoked in a guild
        //         so `self.inner.app_permissions` is present
        unsafe { self.inner.app_permissions.unwrap_unchecked() }
    }

    pub unsafe fn author_permissions_unchecked(&self) -> Permissions {
        // SAFETY: this interaction was invoked in a guild,
        //         so `self.inner.member` is present.
        let member = unsafe { self.member_unchecked() };
        // SAFETY: This member object is sent in an interaction,
        //         so `permissions` is present
        unsafe { member.permissions.unwrap_unchecked() }
    }
}

impl<T: CtxKind> Ctx<T, Guild> {
    pub fn bot_member(&self) -> CacheResult<CachedBotMember> {
        self.cache()
            .member(self.guild_id(), self.bot().user_id())
            .ok_or(Cache)
    }

    pub fn member(&self) -> &PartialMember {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //          so `member` is present
        unsafe { self.member_unchecked() }
    }

    pub fn bot_permissions(&self) -> Permissions {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //          so `app_permissions` is present
        unsafe { self.bot_permissions_unchecked() }
    }

    pub fn bot_permissions_for(&self, channel_id: Id<ChannelMarker>) -> CacheResult<Permissions> {
        if channel_id == self.channel_id() {
            return Ok(self.bot_permissions());
        }

        let guild_id = self.guild_id();
        let user_id = self.bot().user_id();
        let everyone_role = self.cache().role(guild_id.cast()).ok_or(Cache)?;
        let member_roles = self
            .bot_member()?
            .roles()
            .iter()
            .filter(|&&r| r != everyone_role.id)
            .filter_map(|&r| {
                let role = self.cache().role(r)?;

                Some((r, role.permissions))
            })
            .collect::<Box<_>>();

        let channel = self.cache().channel(channel_id).ok_or(Cache)?;
        let channel_overwrites = channel.permission_overwrites.as_ref().ok_or(Cache)?;

        Ok(
            twilight_util::permission_calculator::PermissionCalculator::new(
                guild_id,
                user_id,
                everyone_role.permissions,
                &member_roles,
            )
            .in_channel(channel.kind, channel_overwrites),
        )
    }

    pub fn current_voice_state(&self) -> Option<CachedVoiceStateRef> {
        let user = self.bot().user_id();
        self.cache().voice_state(user, self.guild_id())
    }
}

impl<T: CtxKind, U: CtxLocation> BotStateAware for Ctx<T, U> {
    fn bot(&self) -> &BotState {
        &self.bot
    }
}

impl<T: CtxKind, U: CtxLocation> OwnedBotStateAware for Ctx<T, U> {
    fn bot_owned(&self) -> Arc<BotState> {
        self.bot.clone()
    }
}

impl<T: CtxKind, U: CtxLocation> SenderAware for Ctx<T, U> {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl<T: CtxKind, U: CtxLocation> CacheAware for Ctx<T, U> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl<T: CtxKind, U: CtxLocation> HttpAware for Ctx<T, U> {
    fn http(&self) -> &HttpClient {
        self.bot.http()
    }
}

impl<T: CtxKind, U: CtxLocation> LavalinkAware for Ctx<T, U> {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }
}

impl<T: CtxKind> PlayerAware for GuildCtx<T> {}

impl<T: CtxKind, U: CtxLocation> OptionallyGuildIdAware for Ctx<T, U> {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>> {
        self.inner.guild_id
    }
}

impl<T: CtxKind, U: CtxLocation> AuthorIdAware for Ctx<T, U> {
    fn author_id(&self) -> Id<UserMarker> {
        self.author().id
    }
}

impl<T: CtxKind> GuildIdAware for Ctx<T, Guild> {
    #[inline]
    fn guild_id(&self) -> Id<GuildMarker> {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //         so `self.guild_id_unchecked()` is safe.
        unsafe { self.guild_id_unchecked() }
    }
}

impl<T: CtxKind> AuthorPermissionsAware for Ctx<T, Guild> {
    fn author_permissions(&self) -> Permissions {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //          so `self.author_permissions_unchecked()` is safe.
        unsafe { self.author_permissions_unchecked() }
    }
}

pub struct WeakGuildCtx<'a, T: CtxKind>(&'a Ctx<T>);

impl<'a, T: CtxKind> TryFrom<&'a Ctx<T>> for WeakGuildCtx<'a, T> {
    type Error = NotInGuild;

    fn try_from(value: &'a Ctx<T>) -> Result<Self, Self::Error> {
        value.get_guild_id().ok_or(NotInGuild)?;
        Ok(Self(value))
    }
}

impl<T: CtxKind> GuildIdAware for WeakGuildCtx<'_, T> {
    fn guild_id(&self) -> Id<GuildMarker> {
        // SAFETY: `self.0.get_guild_id()` is proven to be present from `Self::try_from`,
        //         proving that this was an interaction invoked in a guild,
        //         so `self.0.guild_id_unchecked()` is safe.
        unsafe { self.0.guild_id_unchecked() }
    }
}

impl<T: CtxKind> WeakGuildCtx<'_, T> {
    pub fn member(&self) -> &PartialMember {
        // SAFETY: `self.0.get_guild_id()` is proven to be present from `Self::try_from`,
        //          proving that this was an interaction invoked in a guild,
        //          so `self.0.member_unchecked()` is safe.
        unsafe { self.0.member_unchecked() }
    }
}

impl<T: CtxKind> AuthorPermissionsAware for WeakGuildCtx<'_, T> {
    fn author_permissions(&self) -> Permissions {
        // SAFETY: `self.0.get_guild_id()` is proven to be present from `Self::try_from`,
        //          proving that this was an interaction invoked in a guild,
        //          so `self.0.author_permissions_unchecked()` is safe.
        unsafe { self.0.author_permissions_unchecked() }
    }
}
