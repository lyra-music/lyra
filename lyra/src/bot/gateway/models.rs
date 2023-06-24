use anyhow::Result;
use std::{ops::Deref, sync::Arc};
use twilight_cache_inmemory::{model::CachedVoiceState, InMemoryCache};

use async_trait::async_trait;
use twilight_gateway::{stream::ShardRef, Event, Latency, MessageSender};

#[async_trait]
pub trait Process {
    async fn process(self) -> Result<()>;
}

use crate::bot::{
    lavalink::{Lavalink, Lavalinkful},
    lib::models::Lyra,
};

pub struct OldResources {
    pub voice_state: Option<CachedVoiceState>,
}

impl OldResources {
    pub fn new(cache: &InMemoryCache, event: &Event) -> Self {
        let voice_state = match event {
            Event::VoiceStateUpdate(event) => cache
                .voice_state(
                    event.user_id,
                    event
                        .guild_id
                        .expect("`VoiceStateUpdate::guild_id` must exist"),
                )
                .map(|v| v.clone()),
            _ => None,
        };

        Self { voice_state }
    }
}

pub struct ContextedLyra {
    pub event: Event,
    old_resources: OldResources,
    inner: Arc<Lyra>,
    sender: MessageSender,
    latency: Latency,
    lavalink: Arc<Lavalink>,
}

impl ContextedLyra {
    pub fn new(
        event: Event,
        old_resources: OldResources,
        bot: Arc<Lyra>,
        shard: ShardRef,
        lavalink: Arc<Lavalink>,
    ) -> Self {
        Self {
            event,
            old_resources,
            inner: bot,
            sender: shard.sender(),
            latency: shard.latency().clone(),
            lavalink,
        }
    }

    pub fn sender(&self) -> &MessageSender {
        &self.sender
    }

    pub fn latency(&self) -> &Latency {
        &self.latency
    }

    pub fn old_resources(&self) -> &OldResources {
        &self.old_resources
    }
}

impl Deref for ContextedLyra {
    type Target = Lyra;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Lavalinkful for ContextedLyra {
    fn lavalink(&self) -> &Lavalink {
        &self.lavalink
    }
    fn clone_lavalink(&self) -> Arc<Lavalink> {
        self.lavalink.clone()
    }
}
