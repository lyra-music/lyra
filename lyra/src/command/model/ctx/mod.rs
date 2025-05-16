mod autocomplete;
mod command_data;
mod component;
mod defer;
mod followup;
mod menu;
mod message;
mod modal;

use std::{marker::PhantomData, sync::Arc};

use modal::{ModalFromAppCmd, ModalFromComponent};
use tokio::sync::oneshot;
use twilight_cache_inmemory::{InMemoryCache, Reference, model::CachedMember};
use twilight_gateway::{Latency, MessageSender};
use twilight_http::{Client as HttpClient, client::InteractionClient};
use twilight_model::{
    channel::Channel,
    gateway::payload::incoming::InteractionCreate,
    guild::{PartialMember, Permissions},
    id::{
        Id,
        marker::{
            ChannelMarker, GuildMarker as TwilightGuildMarker, InteractionMarker, UserMarker,
        },
    },
    user::User as TwilightUser,
};

use crate::{
    LavalinkAware,
    command::{model::NonPingInteraction, require::CachedVoiceStateRef},
    core::{
        model::{
            BotState, BotStateAware, CacheAware, DatabaseAware, HttpAware, OwnedBotState,
            OwnedBotStateAware, OwnedHttpAware, PartialMemberAware, UserAware,
            UserPermissionsAware, response::Respond,
        },
        r#static::application,
    },
    error::{Cache, CacheResult, NotInGuild},
    gateway::{GuildIdAware, OptionallyGuildIdAware, SenderAware},
    lavalink::Lavalink,
};

use super::PartialInteractionData;

use self::modal::Marker as ModalMarker;
pub use self::{
    autocomplete::Autocomplete,
    defer::DeferCtxKind,
    followup::FollowupCtxKind,
    menu::{Message, User},
    message::RespondVia as RespondViaMessage,
    modal::Guild as GuildModal,
};

type CachedBotMember<'a> = Reference<'a, (Id<TwilightGuildMarker>, Id<UserMarker>), CachedMember>;

pub trait Kind {}

pub trait AppCtxKind {}

pub struct SlashAppMarker;
impl AppCtxKind for SlashAppMarker {}

pub struct AppCtxMarker<T: AppCtxKind>(PhantomData<fn(T) -> T>);
impl<T: AppCtxKind> Kind for AppCtxMarker<T> {}

pub type SlashMarker = AppCtxMarker<SlashAppMarker>;
pub type Slash = Ctx<SlashMarker>;
#[allow(unused)]
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

impl<U: Location> Ctx<ComponentMarker, U> {
    #[allow(unused)]
    pub fn into_modal_interaction(
        self,
        inner: Box<InteractionCreate>,
    ) -> Ctx<ModalFromComponent, U> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            location: self.location,
            data: None,
            acknowledged: false,
            acknowledgement: None,
            kind: PhantomData::<fn(ModalFromComponent) -> ModalFromComponent>,
        }
    }
}

impl<A: AppCtxKind, U: Location> Ctx<AppCtxMarker<A>, U> {
    pub fn into_modal_interaction(self, inner: Box<InteractionCreate>) -> Ctx<ModalFromAppCmd, U> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            location: self.location,
            data: None,
            acknowledged: false,
            acknowledgement: None,
            kind: PhantomData::<fn(ModalFromAppCmd) -> ModalFromAppCmd>,
        }
    }
}

impl<T: Kind, U: Location> Ctx<T, U> {
    pub const fn latency(&self) -> &Latency {
        &self.latency
    }

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn channel(&self) -> &Channel {
        self.inner.channel_expected()
    }

    pub fn interaction_token(&self) -> &str {
        &self.inner.token
    }

    pub fn guild_id_expected(&self) -> Id<TwilightGuildMarker> {
        self.get_guild_id()
            .expect("interactions invoked in a guild must have a guild id")
    }

    pub fn member_expected(&self) -> &PartialMember {
        self.inner
            .member
            .as_ref()
            .expect("interactions invoked in a guild must have a member")
    }

    pub fn author_permissions_expected(&self) -> Permissions {
        self.member_expected()
            .permissions
            .expect("member object sent by interactions must have permissions")
    }
}

impl<T: Kind> Ctx<T, GuildMarker> {
    pub fn bot_member(&self) -> CacheResult<CachedBotMember> {
        self.cache()
            .member(self.guild_id(), self.bot().user_id())
            .ok_or(Cache)
    }

    pub fn bot_permissions(&self) -> Permissions {
        self.inner
            .app_permissions
            .expect("interactions invoked in a guild must have app permissions")
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

impl<T: Kind, U: Location> Respond for Ctx<T, U> {
    fn is_acknowledged(&self) -> bool {
        self.acknowledged
    }

    fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    fn interaction_id(&self) -> Id<InteractionMarker> {
        self.inner.id
    }

    fn interaction_token(&self) -> &str {
        &self.inner.token
    }

    fn interaction_client(&self) -> InteractionClient<'_> {
        self.http().interaction(application::id())
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

impl<T: Kind, U: Location> OwnedHttpAware for Ctx<T, U> {
    fn http_owned(&self) -> Arc<HttpClient> {
        self.bot.http_owned()
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
        self.inner.author_expected()
    }
}

impl<T: Kind> GuildIdAware for Ctx<T, GuildMarker> {
    #[inline]
    fn guild_id(&self) -> Id<TwilightGuildMarker> {
        self.guild_id_expected()
    }
}

impl<T: Kind> PartialMemberAware for Ctx<T, GuildMarker> {
    #[inline]
    fn member(&self) -> &PartialMember {
        self.member_expected()
    }
}

impl<T: Kind> UserPermissionsAware for Ctx<T, GuildMarker> {
    fn user_permissions(&self) -> Permissions {
        self.author_permissions_expected()
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
        self.0.guild_id_expected()
    }
}

impl<T: Kind> GuildRef<'_, T> {
    pub fn member(&self) -> &PartialMember {
        self.0.member_expected()
    }
}

impl<T: Kind> UserPermissionsAware for GuildRef<'_, T> {
    fn user_permissions(&self) -> Permissions {
        self.0.author_permissions_expected()
    }
}
