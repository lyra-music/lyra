use std::{
    env,
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::{request::application::interaction::UpdateResponse, Client, Response};
use twilight_model::{
    application::{command::CommandOptionChoice, interaction::Interaction},
    channel::{
        message::{component::ActionRow, AllowedMentions, Embed, MessageFlags},
        Message,
    },
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{CommandMarker, InteractionMarker, MessageMarker, UserMarker},
        Id,
    },
    oauth::Application,
    user::CurrentUser,
};
use twilight_standby::Standby;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::{
    command::declare::{MESSAGE_COMMANDS, SLASH_COMMANDS, SLASH_COMMANDS_CACHE},
    error::core::{DeserializeBodyFromHttpError, FollowupResult, RespondResult},
    lavalink::{self, Lavalink},
};

pub type MessageResponse = Response<Message>;
pub type UnitRespondResult = RespondResult<()>;
pub type MessageRespondResult = RespondResult<MessageResponse>;
pub type UnitFollowupResult = FollowupResult<()>;
pub type MessageFollowupResult = FollowupResult<MessageResponse>;

pub struct Config {
    pub token: &'static str,
    pub lavalink_host: &'static str,
    pub lavalink_pwd: &'static str,
    pub database_url: &'static str,
}

impl Config {
    pub const fn from_env() -> Self {
        Self {
            token: env!("BOT_TOKEN"),
            lavalink_host: const_str::concat!(env!("SERVER_ADDRESS"), ":", env!("SERVER_PORT")),
            lavalink_pwd: env!("LAVALINK_SERVER_PASSWORD"),
            database_url: env!("DATABASE_URL"),
        }
    }
}

pub struct BotInfo {
    started: DateTime<Utc>,
    guild_count: AtomicUsize,
}

impl BotInfo {
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

pub struct InteractionClient<'a> {
    inner: twilight_http::client::InteractionClient<'a>,
}

pub struct InteractionInterface<'a> {
    inner: InteractionClient<'a>,
    interaction_token: String,
    interaction_id: Id<InteractionMarker>,
}

impl InteractionInterface<'_> {
    fn base_response_data_builder() -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    const fn interaction_token(&self) -> &String {
        &self.interaction_token
    }

    pub async fn respond_with(
        &self,
        data: Option<InteractionResponseData>,
    ) -> MessageRespondResult {
        let interaction_token = self.interaction_token();
        self.inner
            .create_response(
                self.interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data,
                },
            )
            .await?;

        self.inner.response(interaction_token).await
    }

    pub async fn update_message_with(
        &self,
        data: Option<InteractionResponseData>,
    ) -> MessageRespondResult {
        let interaction_token = self.interaction_token();
        self.inner
            .create_response(
                self.interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::UpdateMessage,
                    data,
                },
            )
            .await?;

        self.inner.response(interaction_token).await
    }

    fn update(&self) -> UpdateResponse<'_> {
        self.inner.update_response(self.interaction_token())
    }

    pub async fn update_no_components_embeds(&self, content: &str) -> MessageFollowupResult {
        Ok(self
            .update()
            .components(None)?
            .embeds(None)?
            .content(Some(content))?
            .await?)
    }

    pub async fn update_message_embeds_only(
        &self,
        embeds: impl IntoIterator<Item = Embed> + Send,
    ) -> MessageFollowupResult {
        let data = InteractionResponseDataBuilder::new().embeds(embeds).build();
        Ok(self.update_message_with(Some(data)).await?)
    }

    pub async fn ephem(&self, content: impl Into<String> + Send) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .content(content)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_with(Some(data)).await
    }

    pub async fn followup(&self, content: &str) -> MessageFollowupResult {
        Ok(self
            .inner
            .create_followup(self.interaction_token())
            .content(content)?
            .await?)
    }

    pub async fn followup_ephem(&self, content: &str) -> MessageFollowupResult {
        Ok(self
            .inner
            .create_followup(self.interaction_token())
            .flags(MessageFlags::EPHEMERAL)
            .content(content)?
            .await?)
    }

    pub async fn modal(
        &self,
        custom_id: impl Into<String> + Send,
        title: impl Into<String> + Send,
        text_inputs: impl IntoIterator<Item = impl Into<twilight_model::channel::message::Component>>
            + Send,
    ) -> UnitRespondResult {
        let data = InteractionResponseDataBuilder::new()
            .custom_id(custom_id)
            .title(title)
            .components(text_inputs.into_iter().map(|t| {
                ActionRow {
                    components: vec![t.into()],
                }
                .into()
            }))
            .build()
            .into();

        self.inner
            .create_response(
                self.interaction_id,
                self.interaction_token(),
                &InteractionResponse {
                    kind: InteractionResponseType::Modal,
                    data,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn autocomplete(
        &self,
        choices: impl IntoIterator<Item = CommandOptionChoice> + Send,
    ) -> UnitRespondResult {
        let data = InteractionResponseDataBuilder::new()
            .choices(choices)
            .build()
            .into();

        self.inner
            .create_response(
                self.interaction_id,
                self.interaction_token(),
                &InteractionResponse {
                    kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
                    data,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn update_followup(
        &self,
        message_id: Id<MessageMarker>,
        content: &str,
    ) -> UnitFollowupResult {
        self.inner
            .update_followup(self.interaction_token(), message_id)
            .content(Some(content))?
            .await?;
        Ok(())
    }

    pub async fn delete_followup(&self, message_id: Id<MessageMarker>) -> UnitRespondResult {
        self.inner
            .delete_followup(self.interaction_token(), message_id)
            .await?;
        Ok(())
    }
}

impl<'a> InteractionClient<'a> {
    pub const fn new(client: twilight_http::client::InteractionClient<'a>) -> Self {
        Self { inner: client }
    }

    pub fn interfaces(self, interaction: &Interaction) -> InteractionInterface<'a> {
        InteractionInterface {
            inner: self,
            interaction_token: interaction.token.clone(),
            interaction_id: interaction.id,
        }
    }

    pub async fn register_global_commands(&self) -> UnitRespondResult {
        self.set_global_commands(&[SLASH_COMMANDS.as_ref(), MESSAGE_COMMANDS.as_ref()].concat())
            .await?;

        Ok(())
    }

    pub async fn global_command_id(
        &self,
        name: &str,
    ) -> Result<Id<CommandMarker>, Arc<DeserializeBodyFromHttpError>> {
        Ok(SLASH_COMMANDS_CACHE
            .entry(name.into())
            .or_try_insert_with(async {
                Ok(self
                    .global_commands()
                    .await?
                    .models()
                    .await?
                    .into_iter()
                    .find(|c| c.name == name)
                    .unwrap_or_else(|| panic!("Command `/{name}` must exist"))
                    .id
                    .unwrap_or_else(|| panic!("Command `/{name}` must have an id")))
            })
            .await?
            .into_value())
    }

    pub async fn mention_global_command(
        &self,
        name: Box<str>,
    ) -> Result<Box<str>, Arc<DeserializeBodyFromHttpError>> {
        let id = self.global_command_id(&name).await?;

        Ok(format!("</{name}:{id}>").into_boxed_str())
    }
}

impl<'a> Deref for InteractionClient<'a> {
    type Target = twilight_http::client::InteractionClient<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub type BotStateRef<'a> = &'a BotState;
pub type OwnedBotState = Arc<BotState>;

pub trait AuthorPermissionsAware {
    fn author_permissions(&self) -> Permissions;
}

pub trait BotStateAware {
    fn bot(&self) -> BotStateRef;
}

pub trait OwnedBotStateAware: BotStateAware {
    fn bot_owned(&self) -> OwnedBotState;
}

pub trait CacheAware {
    fn cache(&self) -> &InMemoryCache;
}

pub trait HttpAware {
    fn http(&self) -> &Client;
}

pub struct BotState {
    cache: InMemoryCache,
    http: Client,
    standby: Standby,
    lavalink: Lavalink,
    db: Pool<Postgres>,
    info: BotInfo,
}

impl BotState {
    pub fn new(db: Pool<Postgres>, http: Client, lavalink: Lavalink) -> Self {
        let info = BotInfo {
            started: Utc::now(),
            guild_count: AtomicUsize::default(),
        };

        Self {
            cache: InMemoryCache::new(),
            http,
            standby: Standby::new(),
            lavalink,
            db,
            info,
        }
    }

    pub const fn db(&self) -> &Pool<Postgres> {
        &self.db
    }

    pub const fn standby(&self) -> &Standby {
        &self.standby
    }

    pub const fn info(&self) -> &BotInfo {
        &self.info
    }

    async fn app(&self) -> Result<Application, DeserializeBodyFromHttpError> {
        Ok(self.http.current_user_application().await?.model().await?)
    }

    pub async fn interaction(&self) -> Result<InteractionClient, DeserializeBodyFromHttpError> {
        let client = self.http.interaction(self.app().await?.id);

        Ok(InteractionClient::new(client))
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
}

impl lavalink::ClientAware for BotState {
    fn lavalink(&self) -> &Lavalink {
        &self.lavalink
    }
}

impl CacheAware for BotState {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}

impl CacheAware for Arc<BotState> {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}

impl HttpAware for BotState {
    fn http(&self) -> &Client {
        &self.http
    }
}
