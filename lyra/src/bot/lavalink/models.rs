use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use atomic_option::AtomicOption;
use dashmap::DashMap;
use tokio::sync::RwLock;
use tokio_stream::StreamMap;
use twilight_lavalink::{
    self,
    model::{Destroy, IncomingEvent},
    node::IncomingEvents,
    player::Player,
    Node,
};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker},
    Id,
};

use crate::bot::lib::models::Lyra;

pub trait Lavalinkful {
    fn lavalink(&self) -> &Lavalink;
    fn clone_lavalink(&self) -> Arc<Lavalink>;
}

pub struct ContextedLyra {
    pub event: IncomingEvent,
    inner: Arc<Lyra>,
    lavalink: Arc<Lavalink>,
}

impl ContextedLyra {
    pub fn new(event: IncomingEvent, bot: Arc<Lyra>, lavalink: Arc<Lavalink>) -> Self {
        Self {
            event,
            inner: bot,
            lavalink,
        }
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

pub enum RepeatMode {
    Off,
    Queue,
    Track,
}

pub struct TrackQueue {}

impl TrackQueue {
    pub const fn new() -> Self {
        Self {}
    }
}

pub struct ConnectionInfo {
    channel_id: RwLock<Id<ChannelMarker>>,
    text_channel_id: RwLock<Id<ChannelMarker>>,
    queue: TrackQueue,
    now_playing_message_id: AtomicOption<Id<MessageMarker>>,
    dispatched_connection_change: AtomicBool,
}

impl ConnectionInfo {
    pub fn new(
        channel_id: Id<ChannelMarker>,
        text_channel_id: Id<ChannelMarker>,
    ) -> ConnectionInfo {
        Self {
            channel_id: channel_id.into(),
            queue: TrackQueue::new(),
            text_channel_id: text_channel_id.into(),
            now_playing_message_id: None.into(),
            dispatched_connection_change: false.into(),
        }
    }

    pub async fn channel_id(&self) -> Id<ChannelMarker> {
        *self.channel_id.read().await
    }

    pub async fn text_channel_id(&self) -> Id<ChannelMarker> {
        *self.text_channel_id.read().await
    }

    pub fn dispatched_connection_change(&self) -> bool {
        self.dispatched_connection_change.load(Ordering::SeqCst)
    }

    async fn update_channel_id(&self, channel_id: Id<ChannelMarker>) {
        *self.channel_id.write().await = channel_id
    }
}

pub struct Lavalink {
    client: twilight_lavalink::Lavalink,
    connections: DashMap<Id<GuildMarker>, ConnectionInfo>,
}

impl Lavalink {
    pub fn new(client: twilight_lavalink::Lavalink) -> Self {
        Self {
            client,
            connections: DashMap::new(),
        }
    }

    pub fn connections(&self) -> &DashMap<Id<GuildMarker>, ConnectionInfo> {
        &self.connections
    }

    pub fn new_connection(
        &self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        text_channel_id: Id<ChannelMarker>,
    ) {
        self.connections
            .insert(guild_id, ConnectionInfo::new(channel_id, text_channel_id));
    }

    pub async fn update_connected_channel(
        &self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
    ) {
        self.connections
            .get_mut(&guild_id)
            .unwrap_or_else(|| panic!("value must exist for guild: {}", guild_id))
            .update_channel_id(channel_id)
            .await
    }

    pub fn dispatch_connection_change(&self, guild_id: Id<GuildMarker>) {
        self.connections.alter(&guild_id, |_, v| {
            v.dispatched_connection_change.store(true, Ordering::SeqCst);
            v
        })
    }

    pub fn acknowledge_connection_change(&self, guild_id: Id<GuildMarker>) {
        self.connections.alter(&guild_id, |_, v| {
            v.dispatched_connection_change
                .store(false, Ordering::SeqCst);
            v
        })
    }

    pub fn remove_connection(&self, guild_id: Id<GuildMarker>) {
        self.connections.remove(&guild_id);
    }

    pub async fn create_player(&self, guild_id: Id<GuildMarker>) -> Result<Arc<Player>> {
        Ok(self.player(guild_id).await?)
    }

    pub async fn destroy_player(&self, guild_id: Id<GuildMarker>) -> Result<()> {
        let Some(player) = self.players().get(&guild_id) else {return Ok(())};
        player.send(Destroy::from(guild_id))?;
        Ok(())
    }
}

impl Deref for Lavalink {
    type Target = twilight_lavalink::Lavalink;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

pub struct LavalinkManager {
    nodes: Box<[(Arc<Node>, IncomingEvents)]>,
}

impl LavalinkManager {
    pub fn new(nodes: Box<[(Arc<Node>, IncomingEvents)]>) -> Self {
        Self { nodes }
    }

    pub fn incoming_events(&mut self) -> StreamMap<usize, &mut IncomingEvents> {
        let mut stream_map = StreamMap::new();
        self.nodes.iter_mut().enumerate().for_each(|(n, (_, rx))| {
            stream_map.insert(n, rx);
        });
        stream_map
    }
}
