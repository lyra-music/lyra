use std::sync::Arc;

use twilight_model::gateway::payload::incoming::VoiceStateUpdate;

use twilight_cache_inmemory::{model::CachedVoiceState, InMemoryCache};
use twilight_gateway::MessageSender;
use twilight_http::Client;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::component::{connection, tuning};
use crate::core::model::{BotState, BotStateAware, CacheAware, HttpAware, OwnedBotStateAware};
use crate::error::gateway::ProcessResult;
use crate::{
    gateway::{GuildIdAware, SenderAware},
    lavalink::{Lavalink, LavalinkAware},
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
        // SAFETY: this bot cannot join DM voice calls,
        //         meaning all voice states will be from a guild voice channel,
        //         so `e.guild_id` is present
        unsafe { self.inner.guild_id.unwrap_unchecked() }
    }
}

impl Process for Context {
    async fn process(self) -> ProcessResult {
        connection::handle_voice_state_update(&self).await?;
        tuning::handle_voice_state_update(&self).await?;

        Ok(())
    }
}
