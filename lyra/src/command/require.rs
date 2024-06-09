use lavalink_rs::{
    error::LavalinkResult, model::player::Player as PlayerInfo, player_context::PlayerContext,
};
use twilight_cache_inmemory::{model::CachedVoiceState, InMemoryCache, Reference};
use twilight_model::{
    channel::ChannelType,
    guild::Permissions,
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
};

use crate::{
    core::model::{AuthorIdAware, AuthorPermissionsAware, CacheAware},
    error::{
        command::check, lavalink::NoPlayerError, Cache, CacheResult, InVoiceWithoutSomeoneElse,
        NotInGuild, NotInVoice, QueueEmpty, Suppressed,
    },
    gateway::GuildIdAware,
    lavalink::{PlayerAware, PlayerDataRwLockArc, UnwrappedPlayerData},
};

use super::model::{Ctx, CtxKind, GuildCtx, WeakGuildCtx};

pub fn guild<T: CtxKind>(ctx: Ctx<T>) -> Result<GuildCtx<T>, NotInGuild> {
    GuildCtx::try_from(ctx)
}

pub fn guild_weak<T: CtxKind>(ctx: &Ctx<T>) -> Result<WeakGuildCtx<T>, NotInGuild> {
    WeakGuildCtx::try_from(ctx)
}

pub struct Player {
    pub context: PlayerContext,
}

impl Player {
    pub async fn info(&self) -> LavalinkResult<PlayerInfo> {
        self.context.get_player().await
    }

    pub fn data(&self) -> PlayerDataRwLockArc {
        self.context.data_unwrapped()
    }

    pub async fn and_queue_not_empty(self) -> Result<Self, QueueEmpty> {
        if self.data().read().await.queue().is_empty() {
            return Err(QueueEmpty);
        }

        Ok(self)
    }
}

pub fn player(ctx: &impl PlayerAware) -> Result<Player, NoPlayerError> {
    let context = ctx.get_player().ok_or(NoPlayerError)?;
    Ok(Player { context })
}

pub type CachedVoiceStateRef<'a> =
    Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedVoiceState>;

#[derive(Clone)]
pub struct InVoiceCachedVoiceState {
    guild_id: Id<GuildMarker>,
    channel_id: Id<ChannelMarker>,
    mute: bool,
    suppress: bool,
}

impl From<CachedVoiceStateRef<'_>> for InVoiceCachedVoiceState {
    fn from(value: CachedVoiceStateRef<'_>) -> Self {
        Self {
            guild_id: value.guild_id(),
            channel_id: value.channel_id(),
            mute: value.mute(),
            suppress: value.suppress(),
        }
    }
}

#[must_use]
pub struct InVoice<'a> {
    state: InVoiceCachedVoiceState,
    author_permissions: Permissions,
    pub author_id: Id<UserMarker>,
    pub cache: &'a InMemoryCache,
}

#[must_use]
pub struct PartialInVoice {
    state: InVoiceCachedVoiceState,
    pub author_id: Id<UserMarker>,
}

impl From<&InVoice<'_>> for PartialInVoice {
    fn from(value: &InVoice<'_>) -> Self {
        Self {
            state: value.state.clone(),
            author_id: value.author_id,
        }
    }
}

impl GuildIdAware for PartialInVoice {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.state.guild_id
    }
}

pub fn in_voice<T: CtxKind>(ctx: &GuildCtx<T>) -> Result<InVoice, NotInVoice> {
    let state = ctx.current_voice_state().ok_or(NotInVoice)?;
    // SAFETY: it has been proven that there is a voice connection currently
    Ok(unsafe { InVoice::new(state.into(), ctx) })
}

impl<'a> InVoice<'a> {
    pub unsafe fn new(
        state: InVoiceCachedVoiceState,
        ctx: &'a (impl AuthorPermissionsAware + AuthorIdAware + CacheAware),
    ) -> Self {
        Self {
            state,
            author_permissions: ctx.author_permissions(),
            author_id: ctx.author_id(),
            cache: ctx.cache(),
        }
    }

    pub const fn channel_id(&self) -> Id<ChannelMarker> {
        self.state.channel_id
    }

    pub fn and_unsuppressed(self) -> Result<Self, check::NotSuppressedError> {
        let state = &self.state;
        let voice_state_channel = self.cache.channel(state.channel_id).ok_or(Cache)?;

        if state.mute {
            Err(Suppressed::Muted)?;
        }
        let speaker_in_stage =
            state.suppress && matches!(voice_state_channel.kind, ChannelType::GuildStageVoice);

        if speaker_in_stage {
            Err(Suppressed::NotSpeaker)?;
        }
        Ok(self)
    }

    pub fn and_with_someone_else(self) -> Result<Self, check::InVoiceWithSomeoneElseError> {
        let channel_id = self.channel_id();

        if !someone_else_in(channel_id, &self)? {
            Err(InVoiceWithoutSomeoneElse(channel_id))?;
        }
        Ok(self)
    }
}

impl CacheAware for InVoice<'_> {
    fn cache(&self) -> &InMemoryCache {
        self.cache
    }
}

impl AuthorIdAware for InVoice<'_> {
    fn author_id(&self) -> Id<UserMarker> {
        self.author_id
    }
}

impl AuthorPermissionsAware for InVoice<'_> {
    fn author_permissions(&self) -> Permissions {
        self.author_permissions
    }
}

impl GuildIdAware for InVoice<'_> {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.state.guild_id
    }
}

pub fn someone_else_in(
    channel_id: Id<ChannelMarker>,
    ctx: &(impl CacheAware + AuthorIdAware),
) -> CacheResult<bool> {
    let cache = ctx.cache();
    cache
        .voice_channel_states(channel_id)
        .and_then(|states| {
            for state in states {
                if !cache.user(state.user_id())?.bot && state.user_id() != ctx.author_id() {
                    return Some(true);
                }
            }
            Some(false)
        })
        .ok_or(Cache)
}
