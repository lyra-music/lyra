use std::{num::NonZeroUsize, time::Duration};

use lavalink_rs::{
    error::LavalinkResult, model::player::Player as PlayerInfo, player_context::PlayerContext,
};
use twilight_cache_inmemory::{InMemoryCache, Reference, model::CachedVoiceState};
use twilight_model::{
    channel::ChannelType,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, UserMarker},
    },
};

use crate::{
    LavalinkAndGuildIdAware,
    core::model::{CacheAware, UserIdAware, UserPermissionsAware},
    error::{
        Cache, CacheResult, InVoiceWithoutSomeoneElse, NotInVoice, NotPlaying, QueueEmpty,
        Suppressed,
        command::require::{
            InVoiceWithSomeoneElseError, SeekToWithError, SetPauseWithError, UnsuppressedError,
        },
        lavalink::NoPlayerError,
    },
    gateway::GuildIdAware,
    lavalink::{
        DelegateMethods, OwnedPlayerData, PlayerDataRead, PlayerDataWrite, Queue, QueueItem,
        UnwrappedData,
    },
};

use super::model::{CtxKind, GuildCtx};

pub fn player(cx: &impl LavalinkAndGuildIdAware) -> Result<PlayerInterface, NoPlayerError> {
    let context = cx.get_player().ok_or(NoPlayerError)?;
    Ok(PlayerInterface { context })
}

pub struct PlayerInterface {
    pub context: PlayerContext,
}

impl PlayerInterface {
    pub async fn info(&self) -> LavalinkResult<PlayerInfo> {
        self.context.get_player().await
    }

    pub fn data(&self) -> OwnedPlayerData {
        self.context.data_unwrapped()
    }

    pub async fn update_voice_channel(&self, voice_is_empty: bool) -> LavalinkResult<()> {
        let mut update_player = lavalink_rs::model::http::UpdatePlayer {
            voice: Some(
                self.context
                    .client
                    .get_connection_info_traced(self.context.guild_id)
                    .await?,
            ),
            ..Default::default()
        };
        if voice_is_empty {
            update_player.paused = Some(true);
            self.data().write().await.set_pause(true);
        }
        self.context.update_player(&update_player, true).await?;
        Ok(())
    }

    pub async fn seek_to_with(
        &self,
        timestamp: Duration,
        data_w: &mut PlayerDataWrite<'_>,
    ) -> Result<(), SeekToWithError> {
        data_w.seek_to(timestamp);
        data_w.update_and_apply_now_playing_timestamp().await?;
        self.context.set_position(timestamp).await?;
        Ok(())
    }

    #[inline]
    pub async fn paused(&self) -> bool {
        self.data().read().await.paused()
    }

    pub async fn set_pause(&self, state: bool) -> Result<(), SetPauseWithError> {
        let data = self.data();
        let mut data_w = data.write().await;
        self.set_pause_with(state, &mut data_w).await
    }

    pub async fn set_pause_with(
        &self,
        state: bool,
        data_w: &mut PlayerDataWrite<'_>,
    ) -> Result<(), SetPauseWithError> {
        data_w.set_pause(state);
        data_w.update_and_apply_now_playing_pause(state).await?;
        self.context.set_pause(state).await?;
        Ok(())
    }

    #[inline]
    pub async fn cleanup_now_playing_message_and_play(
        &self,
        cx: &(impl CacheAware + Sync),
        index: usize,
        data_w: &mut PlayerDataWrite<'_>,
    ) -> LavalinkResult<()> {
        cleanup_now_playing_message_and_play(&self.context, cx, index, data_w).await
    }

    pub async fn stop_and_delete_now_playing_message(
        &self,
        data_w: &mut PlayerDataWrite<'_>,
    ) -> LavalinkResult<()> {
        self.context.stop_now().await?;
        data_w.delete_now_playing_message().await;
        Ok(())
    }
}

pub async fn cleanup_now_playing_message_and_play(
    context: &PlayerContext,
    cx: &(impl CacheAware + Sync),
    index: usize,
    data_w: &mut PlayerDataWrite<'_>,
) -> LavalinkResult<()> {
    data_w.cleanup_now_playing_message(cx).await;
    context.play_now(data_w.queue()[index].data()).await?;
    Ok(())
}

pub type CachedVoiceStateRef<'a> =
    Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedVoiceState>;

#[derive(Clone, Debug)]
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
#[derive(Debug, Clone)]
pub struct PartialInVoice {
    state: InVoiceCachedVoiceState,
}

impl PartialInVoice {
    pub const fn channel_id(&self) -> Id<ChannelMarker> {
        self.state.channel_id
    }
}

impl From<&InVoice<'_>> for PartialInVoice {
    fn from(value: &InVoice<'_>) -> Self {
        Self {
            state: value.state.clone(),
        }
    }
}

impl GuildIdAware for PartialInVoice {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.state.guild_id
    }
}

pub fn in_voice<T: CtxKind>(ctx: &'_ GuildCtx<T>) -> Result<InVoice<'_>, NotInVoice> {
    Ok(InVoice::new(
        ctx.current_voice_state().ok_or(NotInVoice)?.into(),
        ctx,
    ))
}

impl<'a> InVoice<'a> {
    pub fn new(
        state: InVoiceCachedVoiceState,
        ctx: &'a (impl UserPermissionsAware + UserIdAware + CacheAware),
    ) -> Self {
        Self {
            state,
            author_permissions: ctx.user_permissions(),
            author_id: ctx.user_id(),
            cache: ctx.cache(),
        }
    }

    pub const fn channel_id(&self) -> Id<ChannelMarker> {
        self.state.channel_id
    }

    pub fn and_unsuppressed(self) -> Result<Self, UnsuppressedError> {
        let state = &self.state;
        let voice_state_channel_kind = self.cache.channel(state.channel_id).ok_or(Cache)?.kind;

        if state.mute {
            return Err(Suppressed::Muted.into());
        }
        let speaker_in_stage =
            state.suppress && matches!(voice_state_channel_kind, ChannelType::GuildStageVoice);

        if speaker_in_stage {
            return Err(Suppressed::NotSpeaker.into());
        }
        Ok(self)
    }

    pub fn and_with_someone_else(self) -> Result<Self, InVoiceWithSomeoneElseError> {
        let channel_id = self.channel_id();

        if !someone_else_in(channel_id, &self)? {
            return Err(InVoiceWithoutSomeoneElse(channel_id).into());
        }
        Ok(self)
    }
}

impl CacheAware for InVoice<'_> {
    fn cache(&self) -> &InMemoryCache {
        self.cache
    }
}

impl UserIdAware for InVoice<'_> {
    fn user_id(&self) -> Id<UserMarker> {
        self.author_id
    }
}

impl UserPermissionsAware for InVoice<'_> {
    fn user_permissions(&self) -> Permissions {
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
    cx: &(impl CacheAware + UserIdAware),
) -> CacheResult<bool> {
    let cache = cx.cache();
    cache
        .voice_channel_states(channel_id)
        .and_then(|states| {
            for state in states {
                if !cache.user(state.user_id())?.bot && state.user_id() != cx.user_id() {
                    return Some(true);
                }
            }
            Some(false)
        })
        .ok_or(Cache)
}

fn impl_queue_not_empty(queue: &Queue) -> Result<(), QueueEmpty> {
    if queue.is_empty() {
        return Err(QueueEmpty);
    }
    Ok(())
}

pub fn queue_not_empty<'guard, 'data, 'borrow>(
    data_r: &'guard PlayerDataRead<'data>,
) -> Result<&'borrow Queue, QueueEmpty>
where
    'guard: 'borrow,
    'data: 'borrow,
{
    let queue = data_r.queue();
    impl_queue_not_empty(queue)?;
    Ok(queue)
}

pub fn queue_not_empty_mut<'guard, 'data, 'borrow>(
    data_w: &'guard mut PlayerDataWrite<'data>,
) -> Result<&'borrow mut Queue, QueueEmpty>
where
    'guard: 'borrow,
    'data: 'borrow,
{
    let queue = data_w.queue_mut();
    impl_queue_not_empty(queue)?;
    Ok(queue)
}

pub fn current_track(queue: &'_ Queue) -> Result<CurrentTrack<'_>, NotPlaying> {
    let (current, position) = queue.current_and_position();
    Ok(CurrentTrack {
        track: current.ok_or(NotPlaying)?,
        position,
    })
}

#[must_use]
pub struct CurrentTrack<'a> {
    pub track: &'a QueueItem,
    pub position: NonZeroUsize,
}
