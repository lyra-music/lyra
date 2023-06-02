use std::ops::Deref;
use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use twilight_cache_inmemory::{
    model::{CachedGuild, CachedMember, CachedVoiceState},
    InMemoryCache,
};
use twilight_http::{Client as HttpClient, Response};
use twilight_lavalink::Lavalink;
use twilight_model::{
    application::interaction::{
        application_command::CommandData, Interaction, InteractionData, InteractionType,
    },
    channel::{
        message::{AllowedMentions, MessageFlags},
        Channel, Message,
    },
    gateway::payload::incoming::InteractionCreate,
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
    user::User,
};
use twilight_util::{
    builder::InteractionResponseDataBuilder, permission_calculator::PermissionCalculator,
};

use super::errors::{Error, Result};
use crate::bot::lib::models::Lyra;
use lyra_proc::declare_kinds;

#[async_trait]
pub trait LyraCommand: Sync + Send {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()>;
}

#[declare_kinds(App, Component, Modal, Autocomplete)]
#[derive(Clone)]
pub struct Context<Kind: ContextKind = App> {
    bot: Arc<Lyra>,
    interaction: Box<InteractionCreate>,
    kind: PhantomData<Kind>,
}

pub trait ContextKind: Sync {}

type RespondResult = anyhow::Result<Response<Message>>;

impl Context {
    pub fn command_data(&self) -> CommandData {
        if let InteractionData::ApplicationCommand(data) = self.interaction_data() {
            return *data.clone();
        }
        unreachable!()
    }
}

impl<Kind: ContextKind> Context<Kind> {
    pub const fn bot(&self) -> &Arc<Lyra> {
        &self.bot
    }

    pub fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }

    pub fn http(&self) -> &HttpClient {
        self.bot.http()
    }

    pub fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }

    pub fn bot_member(&self) -> Result<CachedMember> {
        Ok(self
            .cache()
            .member(self.guild_id_unchecked(), self.bot().user_id())
            .ok_or(Error::Cache)?
            .clone())
    }

    pub fn bot_permissions_for(&self, channel: Id<ChannelMarker>) -> Result<Permissions> {
        let guild_id = self.guild_id_unchecked();
        let everyone_role = self.cache().role(guild_id.cast()).ok_or(Error::Cache)?;
        let bot_roles = self
            .bot_member()?
            .roles()
            .into_par_iter()
            .map(|&r| {
                let role = self.cache().role(r).expect("role must exist in cache");
                (r, role.permissions)
            })
            .collect::<Vec<_>>();
        let channel = self.cache().channel(channel).ok_or(Error::Cache)?;

        let permission_calculator = PermissionCalculator::new(
            guild_id,
            self.bot().user_id(),
            everyone_role.permissions,
            &bot_roles,
        );

        Ok(permission_calculator.in_channel(
            channel.kind,
            channel
                .permission_overwrites
                .as_ref()
                .ok_or(Error::Cache)?
                .as_slice(),
        ))
    }

    pub fn get_guild(&self) -> Option<CachedGuild> {
        self.cache().guild(self.guild_id()?)?.clone().into()
    }

    pub fn guild_id(&self) -> Option<Id<GuildMarker>> {
        self.interaction.guild_id
    }

    pub fn guild_id_unchecked(&self) -> Id<GuildMarker> {
        self.guild_id()
            .expect("this interaction must be executed in guilds")
    }

    pub fn channel(&self) -> &Channel {
        self.interaction
            .channel
            .as_ref()
            .expect("`interaction.channel_id` must not be `None`")
    }

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn author(&self) -> &User {
        self.interaction
            .author()
            .expect("`interaction.author()` must not be `None`")
    }

    #[inline]
    pub fn author_id(&self) -> Id<UserMarker> {
        self.author().id
    }

    fn resp_builder(&self) -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    pub async fn ephem(&self, message: &str) -> RespondResult {
        let data = self
            .resp_builder()
            .content(message)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_rich(Some(data)).await
    }

    pub async fn respond(&self, message: &str) -> RespondResult {
        let data = self.resp_builder().content(message).build();
        self.respond_rich(Some(data)).await
    }

    pub async fn respond_rich(&self, data: Option<InteractionResponseData>) -> RespondResult {
        let client = self.bot().interaction_client().await?;

        client
            .create_response(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data,
                },
            )
            .await?;

        Ok(client.response(&self.interaction.token).await?)
    }

    pub fn interaction(&self) -> Interaction {
        self.interaction.deref().deref().clone()
    }

    pub fn interaction_data(&self) -> &InteractionData {
        self.interaction
            .data
            .as_ref()
            .expect("`interaction.data` must not be `None`")
    }

    pub fn author_permissions(&self) -> Permissions {
        self.interaction
            .member
            .as_ref()
            .expect("this interaction must be executed in guilds")
            .permissions
            .expect("this field should exist")
    }

    pub fn current_voice_state(&self) -> Option<CachedVoiceState> {
        let user = self.bot().user_id();
        self.cache()
            .voice_state(user, self.guild_id_unchecked())
            .as_deref()
            .cloned()
    }
}
