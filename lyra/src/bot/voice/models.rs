use twilight_cache_inmemory::{model::CachedVoiceState, InMemoryCache};
use twilight_model::{
    gateway::payload::incoming::VoiceStateUpdate,
    id::{marker::GuildMarker, Id},
};

use crate::bot::{
    gateway::ContextedLyra,
    lavalink::{Lavalink, Lavalinkful},
    lib::models::Cacheful,
};

pub struct Context<'a> {
    pub inner: &'a VoiceStateUpdate,
    bot: &'a ContextedLyra,
}

impl Context<'_> {
    pub fn old_voice_state(&self) -> Option<&CachedVoiceState> {
        self.bot.old_resources().voice_state.as_ref()
    }

    pub fn guild_id(&self) -> Id<GuildMarker> {
        self.inner
            .guild_id
            .expect("`VoiceStateUpdate::guild_id` must exist")
    }

    pub fn bot(&self) -> &ContextedLyra {
        self.bot
    }
}

impl<'a> Context<'a> {
    pub fn from_voice_state_update(event: &'a VoiceStateUpdate, bot: &'a ContextedLyra) -> Self {
        Self { bot, inner: event }
    }
}

impl Cacheful for Context<'_> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl Lavalinkful for Context<'_> {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }

    fn clone_lavalink(&self) -> std::sync::Arc<Lavalink> {
        self.bot.clone_lavalink()
    }
}
