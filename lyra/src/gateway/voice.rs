use std::sync::Arc;

use futures::TryFutureExt;
use twilight_model::gateway::payload::incoming::VoiceStateUpdate;

use twilight_cache_inmemory::{InMemoryCache, model::CachedVoiceState};
use twilight_gateway::MessageSender;
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

use crate::{
    LavalinkAndGuildIdAware, LavalinkAware,
    component::{connection, playback, tuning},
    core::model::{BotState, BotStateAware, CacheAware, HttpAware, OwnedBotStateAware},
    error::gateway::{ProcessError, ProcessResult},
    gateway::{GuildIdAware, SenderAware},
    lavalink::Lavalink,
};

use super::{LastCachedStates, Process};

pub struct Context {
    pub inner: Box<VoiceStateUpdate>,
    bot: Arc<BotState>,
    states: LastCachedStates,
    sender: MessageSender,
}

impl BotState {
    pub(super) const fn into_voice_state_update_context(
        self: Arc<Self>,
        inner: Box<VoiceStateUpdate>,
        states: LastCachedStates,
        sender: MessageSender,
    ) -> Context {
        Context {
            inner,
            bot: self,
            states,
            sender,
        }
    }
}

impl Context {
    pub const fn old_voice_state(&self) -> Option<&CachedVoiceState> {
        self.states.voice_state.as_ref()
    }
}

impl BotStateAware for Context {
    fn bot(&self) -> &BotState {
        &self.bot
    }
}

impl OwnedBotStateAware for Context {
    fn bot_owned(&self) -> Arc<BotState> {
        self.bot.clone()
    }
}

impl CacheAware for Context {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl LavalinkAware for Context {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }
}

impl HttpAware for Context {
    fn http(&self) -> &Client {
        self.bot.http()
    }
}

impl SenderAware for Context {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl GuildIdAware for Context {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.inner
            .guild_id
            .expect("bots should currently only be able to join guild voice channels")
    }
}

impl Process for Context {
    async fn process(self) -> ProcessResult {
        let mut rx = self
            .get_conn()
            .subscribe_on_change()
            .await
            .expect("failed to subscribe to connection change");

        let connection_changed = tokio::time::timeout(
            crate::core::r#const::connection::CHANGED_TIMEOUT,
            rx.changed(),
        )
        .await
        .is_ok();

        tokio::try_join![
            connection::handle_voice_state_update(&self, connection_changed)
                .map_err(ProcessError::from),
            playback::handle_voice_state_update(&self, connection_changed).map_err(Into::into),
            tuning::handle_voice_state_update(&self).map_err(Into::into),
        ]?;
        Ok(())
    }
}
