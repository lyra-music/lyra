use twilight_cache_inmemory::{model::CachedVoiceState, InMemoryCache};
use twilight_model::id::{marker::GuildMarker, Id};

use twilight_gateway::{Event, MessageSender};

use crate::bot::error::gateway::ProcessResult;

pub trait Process {
    async fn process(self) -> ProcessResult;
}

pub trait SenderAware {
    fn sender(&self) -> &MessageSender;
}

pub trait GuildIdAware {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>>;
}

pub trait ExpectedGuildIdAware {
    fn guild_id(&self) -> Id<GuildMarker>;
}

pub struct LastCachedStates {
    pub voice_state: Option<CachedVoiceState>,
}

impl LastCachedStates {
    pub fn new(cache: &InMemoryCache, event: &Event) -> Self {
        let voice_state = match event {
            Event::VoiceStateUpdate(event) => cache
                .voice_state(
                    event.user_id,
                    event
                        .guild_id
                        .expect("`VoiceStateUpdate::guild_id` must exist"),
                )
                .as_deref()
                .cloned(),
            _ => None,
        };

        Self { voice_state }
    }
}
