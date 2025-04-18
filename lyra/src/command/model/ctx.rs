mod autocomplete;
mod command_data;
mod component;
mod menu;
mod message;
mod modal;

use std::{marker::PhantomData, sync::Arc};

use tokio::sync::oneshot;
use twilight_cache_inmemory::{InMemoryCache, Reference, model::CachedMember};
use twilight_gateway::{Latency, MessageSender};
use twilight_http::Client as HttpClient;
use twilight_model::{
    channel::Channel,
    gateway::payload::incoming::InteractionCreate,
    guild::{PartialMember, Permissions},
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker as TwilightGuildMarker, UserMarker},
    },
    user::User as TwilightUser,
};

use crate::{
    LavalinkAware,
    command::{model::NonPingInteraction, require::CachedVoiceStateRef},
    core::model::{
        BotState, BotStateAware, CacheAware, DatabaseAware, HttpAware, InteractionInterface,
        OwnedBotState, OwnedBotStateAware, PartialMemberAware, UserAware, UserPermissionsAware,
    },
    error::{
        Cache, CacheResult, NotInGuild, command::RespondError, core::DeserialiseBodyFromHttpError,
    },
    gateway::{GuildIdAware, OptionallyGuildIdAware, SenderAware},
    lavalink::Lavalink,
};

use super::PartialInteractionData;

use self::modal::Marker as ModalMarker;
pub use self::{
    autocomplete::Autocomplete,
    command_data::Aware as CommandDataAware,
    menu::{Message, User},
    message::RespondVia as RespondViaMessage,
    modal::{Guild as GuildModal, RespondVia as RespondViaModal},
};

type RespondResult<T> = Result<T, RespondError>;
type UnitRespondResult = RespondResult<()>;
type CachedBotMember<'a> = Reference<'a, (Id<TwilightGuildMarker>, Id<UserMarker>), CachedMember>;

pub trait Kind {}

pub trait AppCtxKind {}

pub struct SlashAppMarker;
impl AppCtxKind for SlashAppMarker {}

pub struct AppCtxMarker<T: AppCtxKind>(PhantomData<fn(T) -> T>);
impl<T: AppCtxKind> Kind for AppCtxMarker<T> {}

pub type SlashMarker = AppCtxMarker<SlashAppMarker>;
pub type Slash = Ctx<SlashMarker>;
pub type GuildSlash = Ctx<SlashMarker, GuildMarker>;

pub struct ComponentMarker;
impl Kind for ComponentMarker {}

pub type Component = Ctx<ComponentMarker>;

pub trait Location {}

pub struct Unknown;
impl Location for Unknown {}

pub struct GuildMarker;
impl Location for GuildMarker {}
pub type Guild<T> = Ctx<T, GuildMarker>;

pub struct Ctx<Of, In = Unknown>
where
    Of: Kind,
    In: Location,
{
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
    data: Option<PartialInteractionData>,
    acknowledged: bool,
    acknowledgement: Option<oneshot::Sender<()>>,
    kind: PhantomData<fn(Of) -> Of>,
    location: PhantomData<fn(In) -> In>,
}

impl<T: Kind> TryFrom<Ctx<T>> for Ctx<T, GuildMarker> {
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
            acknowledgement: value.acknowledgement,
            kind: value.kind,
            location: PhantomData::<fn(GuildMarker) -> GuildMarker>,
        })
    }
}

impl<T: Kind, U: Location> Ctx<T, U> {
    pub fn into_modal_interaction(self, inner: Box<InteractionCreate>) -> Ctx<ModalMarker, U> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            location: self.location,
            data: None,
            acknowledged: false,
            acknowledgement: None,
            kind: PhantomData::<fn(ModalMarker) -> ModalMarker>,
        }
    }

    pub const fn latency(&self) -> &Latency {
        &self.latency
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
        if let Some(tx) = std::mem::take(&mut self.acknowledgement) {
            let _ = tx.send(());
        }
    }

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn channel(&self) -> &Channel {
        // SAFETY: Interaction type is not `Ping`, so `channel` is present.
        unsafe { self.inner.channel_unchecked() }
    }

    pub fn interaction(&self) -> &InteractionCreate {
        self.inner.as_ref()
    }

    pub fn interaction_token(&self) -> &str {
        &self.inner.token
    }

    pub async fn interface(&self) -> Result<InteractionInterface, DeserialiseBodyFromHttpError> {
        Ok(self.bot.interaction().await?.interfaces(&self.inner))
    }

    pub unsafe fn guild_id_unchecked(&self) -> Id<TwilightGuildMarker> {
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

impl<T: Kind> Ctx<T, GuildMarker> {
    pub fn bot_member(&self) -> CacheResult<CachedBotMember> {
        self.cache()
            .member(self.guild_id(), self.bot().user_id())
            .ok_or(Cache)
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

impl<T: Kind, U: Location> BotStateAware for Ctx<T, U> {
    fn bot(&self) -> &BotState {
        &self.bot
    }
}

impl<T: Kind, U: Location> OwnedBotStateAware for Ctx<T, U> {
    fn bot_owned(&self) -> Arc<BotState> {
        self.bot.clone()
    }
}

impl<T: Kind, U: Location> SenderAware for Ctx<T, U> {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl<T: Kind, U: Location> CacheAware for Ctx<T, U> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl<T: Kind, U: Location> HttpAware for Ctx<T, U> {
    fn http(&self) -> &HttpClient {
        self.bot.http()
    }
}

impl<T: Kind, U: Location> LavalinkAware for Ctx<T, U> {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }
}

impl<T: Kind, U: Location> DatabaseAware for Ctx<T, U> {
    fn db(&self) -> &sqlx::Pool<sqlx::Postgres> {
        self.bot.db()
    }
}

impl<T: Kind, U: Location> OptionallyGuildIdAware for Ctx<T, U> {
    fn get_guild_id(&self) -> Option<Id<TwilightGuildMarker>> {
        self.inner.guild_id
    }
}

impl<T: Kind, U: Location> UserAware for Ctx<T, U> {
    #[inline]
    fn user(&self) -> &TwilightUser {
        // SAFETY: Interaction type is not `Ping`, so `author()` is present.
        unsafe { self.inner.author_unchecked() }
    }
}

impl<T: Kind> GuildIdAware for Ctx<T, GuildMarker> {
    #[inline]
    fn guild_id(&self) -> Id<TwilightGuildMarker> {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //         so `self.guild_id_unchecked()` is safe.
        unsafe { self.guild_id_unchecked() }
    }
}

impl<T: Kind> PartialMemberAware for Ctx<T, GuildMarker> {
    #[inline]
    fn member(&self) -> &PartialMember {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //          so `member` is present
        unsafe { self.member_unchecked() }
    }
}

impl<T: Kind> UserPermissionsAware for Ctx<T, GuildMarker> {
    fn user_permissions(&self) -> Permissions {
        // SAFETY: `Ctx<_, Guild>` is proven to be of an interaction that was invoked in a guild,
        //          so `self.author_permissions_unchecked()` is safe.
        unsafe { self.author_permissions_unchecked() }
    }
}

pub struct GuildRef<'a, T: Kind>(&'a Ctx<T>);

impl<'a, T: Kind> TryFrom<&'a Ctx<T>> for GuildRef<'a, T> {
    type Error = NotInGuild;

    fn try_from(value: &'a Ctx<T>) -> Result<Self, Self::Error> {
        value.get_guild_id().ok_or(NotInGuild)?;
        Ok(Self(value))
    }
}

impl<T: Kind> GuildIdAware for GuildRef<'_, T> {
    fn guild_id(&self) -> Id<TwilightGuildMarker> {
        // SAFETY: `self.0.get_guild_id()` is proven to be present from `Self::try_from`,
        //         proving that this was an interaction invoked in a guild,
        //         so `self.0.guild_id_unchecked()` is safe.
        unsafe { self.0.guild_id_unchecked() }
    }
}

impl<T: Kind> GuildRef<'_, T> {
    pub fn member(&self) -> &PartialMember {
        // SAFETY: `self.0.get_guild_id()` is proven to be present from `Self::try_from`,
        //          proving that this was an interaction invoked in a guild,
        //          so `self.0.member_unchecked()` is safe.
        unsafe { self.0.member_unchecked() }
    }
}

impl<T: Kind> UserPermissionsAware for GuildRef<'_, T> {
    fn user_permissions(&self) -> Permissions {
        // SAFETY: `self.0.get_guild_id()` is proven to be present from `Self::try_from`,
        //          proving that this was an interaction invoked in a guild,
        //          so `self.0.author_permissions_unchecked()` is safe.
        unsafe { self.0.author_permissions_unchecked() }
    }
}
