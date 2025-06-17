mod autocomplete;
mod command_data;
mod component;
mod defer;
mod followup;
mod menu;
mod message;
mod modal;

use std::{marker::PhantomData, sync::Arc};

use modal::{CmdModalMarker, ComponentModalMarker};
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
        statik::application,
    },
    error::{Cache, CacheResult},
    gateway::{GuildIdAware, OptionallyGuildIdAware, SenderAware},
    lavalink::Lavalink,
};

use super::PartialInteractionData;

use self::modal::ModalMarker;
pub use self::{
    autocomplete::{AutocompleteCtx, GuildAutocompleteCtx},
    defer::RespondWithDeferKind,
    followup::FollowupKind,
    menu::{GuildMessageCmdCtx, MessageCmdCtx, UserCmdCtx},
    message::RespondWithMessageKind,
    modal::GuildModalCtx,
};

type CachedBotMember<'a> = Reference<'a, (Id<TwilightGuildMarker>, Id<UserMarker>), CachedMember>;

pub trait CtxKind {}

pub trait CmdInnerMarkerKind {}

pub struct SlashCmdInnerMarker;
impl CmdInnerMarkerKind for SlashCmdInnerMarker {}

pub struct CmdMarker<T: CmdInnerMarkerKind>(PhantomData<fn(T) -> T>);
impl<T: CmdInnerMarkerKind> CtxKind for CmdMarker<T> {}

pub type SlashCmdMarker = CmdMarker<SlashCmdInnerMarker>;
pub type SlashCmdCtx = Ctx<SlashCmdMarker>;
pub type GuildSlashCmdCtx = Ctx<SlashCmdMarker, GuildMarker>;

pub struct ComponentMarker;
impl CtxKind for ComponentMarker {}

#[expect(unused)]
pub type ComponentCtx = Ctx<ComponentMarker>;
pub type GuildComponentCtx = GuildCtx<ComponentMarker>;

pub trait CtxContext {}

pub struct NonGuildMarker;
impl CtxContext for NonGuildMarker {}

pub struct GuildMarker;
impl CtxContext for GuildMarker {}
pub type GuildCtx<T> = Ctx<T, GuildMarker>;

pub struct Ctx<T, C = NonGuildMarker>
where
    T: CtxKind,
    C: CtxContext,
{
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
    data: Option<PartialInteractionData>,
    acknowledged: bool,
    acknowledgement: Option<oneshot::Sender<()>>,
    kind: PhantomData<fn(T) -> T>,
    context: PhantomData<fn(C) -> C>,
}

impl<C: CtxContext> Ctx<ComponentMarker, C> {
    #[expect(unused)]
    pub fn into_modal_interaction(
        self,
        inner: Box<InteractionCreate>,
    ) -> Ctx<ComponentModalMarker, C> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            context: self.context,
            data: None,
            acknowledged: false,
            acknowledgement: None,
            kind: PhantomData::<fn(ComponentModalMarker) -> ComponentModalMarker>,
        }
    }
}

impl<A: CmdInnerMarkerKind, C: CtxContext> Ctx<CmdMarker<A>, C> {
    pub fn into_modal_interaction(self, inner: Box<InteractionCreate>) -> Ctx<CmdModalMarker, C> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            context: self.context,
            data: None,
            acknowledged: false,
            acknowledgement: None,
            kind: PhantomData::<fn(CmdModalMarker) -> CmdModalMarker>,
        }
    }
}

impl<T: CtxKind, C: CtxContext> Ctx<T, C> {
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

impl<T: CtxKind> Ctx<T, GuildMarker> {
    #[inline]
    pub fn cast_as_non_guild(self) -> Ctx<T, NonGuildMarker> {
        Ctx {
            inner: self.inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            data: self.data,
            acknowledged: self.acknowledged,
            acknowledgement: self.acknowledgement,
            kind: self.kind,
            context: PhantomData::<fn(NonGuildMarker) -> NonGuildMarker>,
        }
    }

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

impl<T: CtxKind, C: CtxContext> Respond for Ctx<T, C> {
    fn is_acknowledged(&self) -> bool {
        self.acknowledged
    }

    fn acknowledge(&mut self) {
        self.acknowledged = true;
        if let Some(tx) = std::mem::take(&mut self.acknowledgement) {
            let _ = tx.send(());
        }
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

impl<T: CtxKind, C: CtxContext> BotStateAware for Ctx<T, C> {
    fn bot(&self) -> &BotState {
        &self.bot
    }
}

impl<T: CtxKind, C: CtxContext> OwnedBotStateAware for Ctx<T, C> {
    fn bot_owned(&self) -> Arc<BotState> {
        self.bot.clone()
    }
}

impl<T: CtxKind, C: CtxContext> SenderAware for Ctx<T, C> {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl<T: CtxKind, C: CtxContext> CacheAware for Ctx<T, C> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl<T: CtxKind, C: CtxContext> HttpAware for Ctx<T, C> {
    fn http(&self) -> &HttpClient {
        self.bot.http()
    }
}

impl<T: CtxKind, C: CtxContext> OwnedHttpAware for Ctx<T, C> {
    fn http_owned(&self) -> Arc<HttpClient> {
        self.bot.http_owned()
    }
}

impl<T: CtxKind, C: CtxContext> LavalinkAware for Ctx<T, C> {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }
}

impl<T: CtxKind, C: CtxContext> DatabaseAware for Ctx<T, C> {
    fn db(&self) -> &sqlx::Pool<sqlx::Postgres> {
        self.bot.db()
    }
}

impl<T: CtxKind, C: CtxContext> OptionallyGuildIdAware for Ctx<T, C> {
    fn get_guild_id(&self) -> Option<Id<TwilightGuildMarker>> {
        self.inner.guild_id
    }
}

impl<T: CtxKind, C: CtxContext> UserAware for Ctx<T, C> {
    #[inline]
    fn user(&self) -> &TwilightUser {
        self.inner.author_expected()
    }
}

impl<T: CtxKind> GuildIdAware for Ctx<T, GuildMarker> {
    #[inline]
    fn guild_id(&self) -> Id<TwilightGuildMarker> {
        self.guild_id_expected()
    }
}

impl<T: CtxKind> PartialMemberAware for Ctx<T, GuildMarker> {
    #[inline]
    fn member(&self) -> &PartialMember {
        self.member_expected()
    }
}

impl<T: CtxKind> UserPermissionsAware for Ctx<T, GuildMarker> {
    fn user_permissions(&self) -> Permissions {
        self.author_permissions_expected()
    }
}
