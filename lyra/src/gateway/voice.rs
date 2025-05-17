use std::sync::Arc;

use futures::{FutureExt, TryFutureExt};
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
    #[tracing::instrument(skip_all, name = "voice_state_update")]
    async fn process(self) -> ProcessResult {
        let get_conn = self.get_conn();
        let disabled = get_conn.vsu_handler_disabled().await;

        let mut vec = Vec::new();
        if let Ok(h) = get_conn.get_head().await {
            let t = tuning::handle_voice_state_update(&self, h.clone()).map_err(ProcessError::from);
            vec.push(t.boxed());

            if disabled {
                tracing::debug!("voice state update handler is disabled");
            } else {
                let c = connection::handle_voice_state_update(&self, h.clone()).map_err(Into::into);
                let p = playback::handle_voice_state_update(&self, h).map_err(Into::into);
                vec.extend([c.boxed(), p.boxed()]);
            }
        } else {
            tracing::debug!("no active connection");
        }

        futures::future::try_join_all(vec).await?;
        Ok(())
    }
}
