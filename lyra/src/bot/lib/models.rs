use std::{
    env,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use hyper::client::{Client as HyperClient, HttpConnector};
use log::LevelFilter;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, Pool, Postgres,
};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::{client::InteractionClient, Client as HttpClient, Response};
use twilight_model::{
    application::interaction::Interaction,
    channel::{
        message::{AllowedMentions, MessageFlags},
        Message,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::UserMarker, Id},
    oauth::Application,
    user::CurrentUser,
};
use twilight_standby::Standby;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::commands::declare::COMMANDS;

pub type RespondResult = Result<Response<Message>>;

pub trait Cacheful {
    fn cache(&self) -> &InMemoryCache;
}

#[derive(Clone)]
pub struct Config {
    pub token: String,
    pub lavalink_addr: SocketAddr,
    pub lavalink_auth: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            token: env::var("BOT_TOKEN").expect("`BOT_TOKEN` must be set"),
            lavalink_addr: SocketAddr::from_str(
                env::var("LAVALINK_ADDR")
                    .expect("`LAVALINK_ADDR` must be set")
                    .as_str(),
            )
            .expect("`LAVALINK_ADDR` must be a valid address"),
            lavalink_auth: env::var("LAVALINK_AUTH").expect("`LAVALINK_AUTH` must be set"),
            database_url: env::var("DATABASE_URL").expect("`DATABASE_URL` must be set"),
        }
    }
}

pub struct LyraInfo {
    started: DateTime<Utc>,
    guild_count: AtomicUsize,
}

impl LyraInfo {
    pub fn guild_count(&self) -> usize {
        self.guild_count.load(Ordering::Relaxed)
    }

    pub fn set_guild_count(&self, guild_count: usize) {
        self.guild_count.store(guild_count, Ordering::Relaxed);
    }

    pub fn increment_guild_count(&self) {
        self.guild_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_guild_count(&self) {
        self.guild_count.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct Lyra {
    config: Config,
    cache: Arc<InMemoryCache>,
    http: Arc<HttpClient>,
    db: Pool<Postgres>,
    hyper: HyperClient<HttpConnector>,
    standby: Standby,
    info: LyraInfo,
}

impl Lyra {
    pub async fn new(config: Config, http: Arc<HttpClient>) -> Result<Self> {
        let Config {
            ref database_url, ..
        } = config;

        let mut options = PgConnectOptions::from_str(database_url.as_str())?;
        options.log_statements(LevelFilter::Debug);
        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let info = LyraInfo {
            started: Utc::now(),
            guild_count: AtomicUsize::new(0),
        };

        Ok(Self {
            config,
            cache: InMemoryCache::new().into(),
            http,
            db,
            hyper: HyperClient::new(),
            standby: Standby::new(),
            info,
        })
    }

    pub fn clone_cache(&self) -> Arc<InMemoryCache> {
        self.cache.clone()
    }

    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    pub fn clone_http(&self) -> Arc<HttpClient> {
        self.http.clone()
    }

    pub const fn hyper(&self) -> &HyperClient<HttpConnector> {
        &self.hyper
    }

    pub const fn db(&self) -> &Pool<Postgres> {
        &self.db
    }

    pub const fn standby(&self) -> &Standby {
        &self.standby
    }

    pub const fn info(&self) -> &LyraInfo {
        &self.info
    }

    pub async fn app(&self) -> Result<Application> {
        Ok(self.http.current_user_application().await?.model().await?)
    }

    pub async fn interaction_client(&self) -> Result<InteractionClient> {
        Ok(self.http.interaction(self.app().await?.id))
    }

    pub async fn register_app_commands(&self) -> Result<()> {
        let client = self.interaction_client().await?;

        client.set_global_commands(COMMANDS.as_ref()).await?;

        Ok(())
    }

    pub fn user(&self) -> CurrentUser {
        self.cache
            .current_user()
            .expect("current user object must be available")
    }

    #[inline]
    pub fn user_id(&self) -> Id<UserMarker> {
        self.user().id
    }

    pub fn base_interaction_response_data_builder() -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    pub async fn ephem_to(
        &self,
        interaction: &Interaction,
        content: impl Into<String>,
    ) -> RespondResult {
        let data = Self::base_interaction_response_data_builder()
            .content(content)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_rich_to(interaction, Some(data)).await
    }

    pub async fn respond_to(
        &self,
        interaction: &Interaction,
        content: impl Into<String>,
    ) -> RespondResult {
        let data = Self::base_interaction_response_data_builder()
            .content(content)
            .build();
        self.respond_rich_to(interaction, Some(data)).await
    }

    pub async fn respond_rich_to(
        &self,
        interaction: &Interaction,
        data: Option<InteractionResponseData>,
    ) -> RespondResult {
        let client = self.interaction_client().await?;

        client
            .create_response(
                interaction.id,
                &interaction.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data,
                },
            )
            .await?;

        Ok(client.response(&interaction.token).await?)
    }
}

impl Cacheful for Lyra {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}
