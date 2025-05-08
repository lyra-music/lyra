use std::sync::Arc;

use lavalink_rs::{
    client::LavalinkClient, error::LavalinkResult, model::player::ConnectionInfo,
    prelude::PlayerContext,
};
use tokio::sync::RwLock;
use twilight_model::id::{Id, marker::ChannelMarker};

use crate::core::r#const;

use super::{Lavalink, OwnedClientData, OwnedPlayerData, RawPlayerData, UnwrappedData};

type LavalinkGuildId = lavalink_rs::model::GuildId;

pub trait DelegateMethods {
    fn handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    );
    fn handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    );
    fn process(&self, event: &twilight_gateway::Event) {
        match event {
            twilight_gateway::Event::VoiceServerUpdate(e) => {
                self.handle_voice_server_update(e.guild_id, e.token.clone(), e.endpoint.clone());
            }
            twilight_gateway::Event::VoiceStateUpdate(e) => {
                self.handle_voice_state_update(
                    e.guild_id
                        .expect("bots should currently only be able to join guild voice channels"),
                    e.channel_id,
                    e.user_id,
                    e.session_id.clone(),
                );
            }
            _ => {}
        }
    }

    async fn get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo>;
    async fn get_connection_info_traced(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> LavalinkResult<ConnectionInfo> {
        let now = tokio::time::Instant::now();
        let info = self
            .get_connection_info(
                guild_id,
                r#const::connection::GET_LAVALINK_CONNECTION_INFO_TIMEOUT,
            )
            .await?;
        tracing::debug!("getting lavalink connection info took {:?}", now.elapsed());
        Ok(info)
    }

    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext>;
    async fn new_player(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send + Copy,
        channel_id: Id<ChannelMarker>,
    ) -> LavalinkResult<PlayerContext> {
        let info = self.get_connection_info_traced(guild_id).await?;
        let data = Arc::new(RwLock::new(RawPlayerData::new(channel_id)));
        let player = self
            .create_player_context_with_data(guild_id, info, data)
            .await?;

        Ok(player)
    }

    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext>;
    fn get_player_data(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> Option<OwnedPlayerData> {
        self.get_player_context(guild_id)
            .map(|c| c.data_unwrapped())
    }
    fn data(&self) -> OwnedClientData;
}

impl DelegateMethods for LavalinkClient {
    #[inline]
    fn handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    ) {
        self.handle_voice_server_update(guild_id, token, endpoint);
    }

    #[inline]
    fn handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    ) {
        self.handle_voice_state_update(guild_id, channel_id, user_id, session_id);
    }

    #[inline]
    async fn get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo> {
        self.get_connection_info(guild_id, timeout).await
    }

    #[inline]
    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext> {
        self.create_player_context_with_data(guild_id, connection_info, user_data)
            .await
    }

    #[inline]
    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext> {
        self.get_player_context(guild_id)
    }

    #[inline]
    fn data(&self) -> OwnedClientData {
        self.data_unwrapped()
    }
}

impl DelegateMethods for Lavalink {
    #[inline]
    fn handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    ) {
        <LavalinkClient as DelegateMethods>::handle_voice_server_update(
            &self.inner,
            guild_id,
            token,
            endpoint,
        );
    }

    #[inline]
    fn handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    ) {
        <LavalinkClient as DelegateMethods>::handle_voice_state_update(
            &self.inner,
            guild_id,
            channel_id,
            user_id,
            session_id,
        );
    }

    #[inline]
    async fn get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo> {
        <LavalinkClient as DelegateMethods>::get_connection_info(&self.inner, guild_id, timeout)
            .await
    }

    #[inline]
    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext> {
        <LavalinkClient as DelegateMethods>::create_player_context_with_data(
            &self.inner,
            guild_id,
            connection_info,
            user_data,
        )
        .await
    }

    #[inline]
    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext> {
        <LavalinkClient as DelegateMethods>::get_player_context(&self.inner, guild_id)
    }

    #[inline]
    fn data(&self) -> OwnedClientData {
        self.inner.data_unwrapped()
    }
}
