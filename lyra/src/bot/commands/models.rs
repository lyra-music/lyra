use std::{marker::PhantomData, ops::Deref, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use twilight_cache_inmemory::{
    model::{CachedGuild, CachedMember, CachedVoiceState},
    InMemoryCache,
};
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::interaction::{
        application_command::CommandData, Interaction, InteractionData, InteractionType,
    },
    channel::{
        message::{component::ActionRow, Embed, MessageFlags},
        Channel,
    },
    gateway::payload::incoming::InteractionCreate,
    guild::{PartialMember, Permissions},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ChannelMarker, CommandMarker, GuildMarker, UserMarker},
        Id,
    },
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::errors;
use crate::bot::{
    gateway::ContextedLyra,
    lavalink::{Lavalink, Lavalinkful},
    lib::models::{Cacheful, Lyra, RespondResult},
};
use lyra_proc::declare_kinds;

pub trait ResolvedCommandInfo {
    fn id() -> Id<CommandMarker>;
    fn name() -> String;
}

#[async_trait]
pub trait LyraCommand: ResolvedCommandInfo {
    async fn execute(self, ctx: Context<App>) -> Result<()>;
}

#[declare_kinds(App, Component, Modal, Autocomplete)]
#[derive(Clone)]
pub struct Context<Kind: ContextKind> {
    inner: Box<InteractionCreate>,
    bot: Arc<ContextedLyra>,
    kind: PhantomData<Kind>,
}

pub trait ContextKind: Sync {}

impl Context<App> {
    pub fn command_data(&self) -> &CommandData {
        if let InteractionData::ApplicationCommand(data) = self.interaction_data() {
            return data;
        }
        unreachable!()
    }
}

impl<Kind: ContextKind> Context<Kind> {
    pub fn bot(&self) -> &ContextedLyra {
        &self.bot
    }

    pub fn http(&self) -> &HttpClient {
        self.bot.http()
    }

    pub fn db(&self) -> &Pool<Postgres> {
        self.bot.db()
    }

    pub fn bot_member(&self) -> errors::Result<CachedMember> {
        Ok(self
            .cache()
            .member(self.guild_id_unchecked(), self.bot().user_id())
            .ok_or(errors::Error::Cache)?
            .clone())
    }

    pub fn get_guild(&self) -> Option<CachedGuild> {
        self.cache().guild(self.guild_id()?)?.clone().into()
    }

    pub fn guild_id(&self) -> Option<Id<GuildMarker>> {
        self.inner.guild_id
    }

    pub fn guild_id_unchecked(&self) -> Id<GuildMarker> {
        self.guild_id()
            .expect("this interaction must be executed in guilds")
    }

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn channel(&self) -> &Channel {
        self.inner
            .channel
            .as_ref()
            .expect("`self.inner.channel` must not be `None`")
    }

    pub fn author(&self) -> &User {
        self.inner
            .author()
            .expect("`self.inner.author()` must not be `None`")
    }

    pub fn member(&self) -> &PartialMember {
        self.inner
            .member
            .as_ref()
            .expect("`self.inner.member` must not be `None`")
    }

    #[inline]
    pub fn author_id(&self) -> Id<UserMarker> {
        self.author().id
    }

    fn base_interaction_response_data_builder() -> InteractionResponseDataBuilder {
        Lyra::base_interaction_response_data_builder()
    }

    pub async fn ephem(&self, content: impl Into<String>) -> RespondResult {
        self.bot.ephem_to(&self.inner, content).await
    }

    pub async fn respond(&self, content: impl Into<String>) -> RespondResult {
        self.bot.respond_to(&self.inner, content).await
    }

    pub async fn respond_rich(&self, data: Option<InteractionResponseData>) -> RespondResult {
        self.bot.respond_rich_to(&self.inner, data).await
    }

    pub async fn respond_components(
        &self,
        content: impl Into<String>,
        components: impl IntoIterator<Item = twilight_model::channel::message::Component>,
    ) -> RespondResult {
        let data = Self::base_interaction_response_data_builder()
            .content(content)
            .components(components)
            .build();
        self.respond_rich(Some(data)).await
    }

    pub async fn respond_modal(
        &self,
        custom_id: impl Into<String>,
        title: impl Into<String>,
        text_inputs: impl IntoIterator<Item = impl Into<twilight_model::channel::message::Component>>,
    ) -> Result<()> {
        let client = self.bot().interaction_client().await?;

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

        client
            .create_response(
                self.inner.id,
                &self.inner.token,
                &InteractionResponse {
                    kind: InteractionResponseType::Modal,
                    data,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn respond_embeds_only(
        &self,
        embeds: impl IntoIterator<Item = Embed>,
    ) -> RespondResult {
        let data = Self::base_interaction_response_data_builder()
            .embeds(embeds)
            .build();
        self.respond_rich(Some(data)).await
    }

    pub async fn followup(&self, content: &str) -> RespondResult {
        let client = self.bot().interaction_client().await?;

        Ok(client
            .create_followup(&self.inner.token)
            .content(content)?
            .await?)
    }

    pub async fn followup_ephem(&self, content: &str) -> RespondResult {
        let client = self.bot().interaction_client().await?;

        Ok(client
            .create_followup(&self.inner.token)
            .flags(MessageFlags::EPHEMERAL)
            .content(content)?
            .await?)
    }

    pub async fn update_response(&self, content: &str) -> RespondResult {
        let client = self.bot().interaction_client().await?;

        Ok(client
            .update_response(&self.inner.token)
            .content(content.into())?
            .await?)
    }

    pub fn interaction(&self) -> Interaction {
        self.inner.deref().deref().clone()
    }

    pub fn interaction_data(&self) -> &InteractionData {
        self.inner
            .data
            .as_ref()
            .expect("`interaction.data` must not be `None`")
    }

    pub fn author_permissions(&self) -> Permissions {
        self.inner
            .member
            .as_ref()
            .expect("this interaction must be executed in guilds")
            .permissions
            .expect("this field should exist")
    }

    pub fn bot_permissions(&self) -> Permissions {
        self.inner
            .app_permissions
            .expect("this interaction must be executed in guilds")
    }

    pub fn current_voice_state(&self) -> Option<CachedVoiceState> {
        let user = self.bot().user_id();
        self.cache()
            .voice_state(user, self.guild_id()?)
            .as_deref()
            .cloned()
    }
}

impl<Kind: ContextKind> Cacheful for Context<Kind> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl<Kind: ContextKind> Lavalinkful for Context<Kind> {
    fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }

    fn clone_lavalink(&self) -> Arc<Lavalink> {
        self.bot.clone_lavalink()
    }
}
