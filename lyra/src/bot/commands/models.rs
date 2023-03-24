use std::ops::Deref;
use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use twilight_http::{Client as HttpClient, Response};
use twilight_lavalink::Lavalink;
use twilight_model::{
    application::interaction::{
        application_command::CommandData, Interaction, InteractionData, InteractionType,
    },
    channel::{
        message::{AllowedMentions, MessageFlags},
        Message,
    },
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::lib::models::LyraBot;
use lyra_proc::declare_kinds;

#[async_trait]
pub trait LyraCommand: Sync + Send {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()>;
}

#[declare_kinds(App, Component, Modal, Autocomplete)]
pub struct Context<Kind = App>
where
    Kind: ContextKind,
{
    bot: Arc<LyraBot>,
    interaction: Box<InteractionCreate>,
    kind: PhantomData<Kind>,
}

pub trait ContextKind {}

type RespondResult = anyhow::Result<Response<Message>>;

impl Context<App> {
    pub fn command_data(&self) -> CommandData {
        if let InteractionData::ApplicationCommand(data) = self.interaction_data() {
            return *data.clone();
        }
        unreachable!()
    }
}

impl<Kind> Context<Kind>
where
    Kind: ContextKind,
{
    pub fn bot(&self) -> &Arc<LyraBot> {
        &self.bot
    }

    pub fn http(&self) -> &HttpClient {
        &self.bot.http()
    }

    pub fn lavalink(&self) -> &Lavalink {
        self.bot.lavalink()
    }

    pub fn guild_id(&self) -> &Option<Id<GuildMarker>> {
        &self.interaction.guild_id
    }

    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.interaction
            .channel_id
            .expect("`interaction.channel_id` must not be `None`")
    }

    pub fn author(&self) -> &User {
        &self
            .interaction
            .author()
            .expect("`interaction.author()` must not be `None`")
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
        Ok(self.respond_rich(Some(data)).await?)
    }

    pub async fn respond(&self, message: &str) -> RespondResult {
        let data = self.resp_builder().content(message).build();
        Ok(self.respond_rich(Some(data)).await?)
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
}
