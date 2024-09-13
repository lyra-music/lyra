use std::fmt::{Display, Write};

use twilight_http::{request::application::interaction::UpdateResponse, Response};
use twilight_model::{
    application::{command::CommandOptionChoice, interaction::Interaction},
    channel::{
        message::{component::ActionRow, AllowedMentions, Embed, MessageFlags},
        Message,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{CommandMarker, InteractionMarker, MessageMarker},
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{
    command::{
        declare::{MESSAGE_COMMANDS, POPULATED_COMMANDS_MAP, SLASH_COMMANDS},
        model::CommandInfoAware,
    },
    error::core::{FollowupResult, RegisterGlobalCommandsError, RespondResult},
};

pub type MessageResponse = Response<Message>;
pub type UnitRespondResult = RespondResult<()>;
pub type MessageRespondResult = RespondResult<MessageResponse>;
pub type UnitFollowupResult = FollowupResult<()>;
pub type MessageFollowupResult = FollowupResult<MessageResponse>;

pub struct Client<'a>(twilight_http::client::InteractionClient<'a>);

pub struct Interface<'a> {
    inner: Client<'a>,
    interaction_token: String,
    interaction_id: Id<InteractionMarker>,
}

impl Interface<'_> {
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
            .0
            .create_response(
                self.interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data,
                },
            )
            .await?;

        self.inner.0.response(interaction_token).await
    }

    pub async fn update_message_with(
        &self,
        data: Option<InteractionResponseData>,
    ) -> MessageRespondResult {
        let interaction_token = self.interaction_token();
        self.inner
            .0
            .create_response(
                self.interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::UpdateMessage,
                    data,
                },
            )
            .await?;

        self.inner.0.response(interaction_token).await
    }

    fn update(&self) -> UpdateResponse<'_> {
        self.inner.0.update_response(self.interaction_token())
    }

    pub async fn update_no_components_embeds(
        &self,
        content: impl Into<String> + Send,
    ) -> MessageRespondResult {
        self.update()
            .components(None)
            .embeds(None)
            .content(Some(&content.into()))
            .await
    }

    pub async fn update_message_embeds_only(
        &self,
        embeds: impl IntoIterator<Item = Embed> + Send,
    ) -> MessageFollowupResult {
        let data = InteractionResponseDataBuilder::new().embeds(embeds).build();
        Ok(self.update_message_with(Some(data)).await?)
    }

    pub async fn respond(&self, content: impl Into<String> + Send) -> MessageRespondResult {
        let data = Self::base_response_data_builder().content(content).build();
        self.respond_with(Some(data)).await
    }

    pub async fn respond_ephemeral(
        &self,
        content: impl Into<String> + Send,
    ) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .content(content)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_with(Some(data)).await
    }

    pub async fn followup(&self, content: impl Into<String> + Send) -> MessageFollowupResult {
        Ok(self
            .inner
            .0
            .create_followup(self.interaction_token())
            .content(&content.into())
            .await?)
    }

    pub async fn followup_ephemeral(
        &self,
        content: impl Into<String> + Send,
    ) -> MessageFollowupResult {
        Ok(self
            .inner
            .0
            .create_followup(self.interaction_token())
            .flags(MessageFlags::EPHEMERAL)
            .content(&content.into())
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
            .0
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
            .0
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

    async fn defer_as(&self, ephemeral: bool) -> UnitRespondResult {
        let mut data = Self::base_response_data_builder();
        if ephemeral {
            data = data.flags(MessageFlags::EPHEMERAL);
        }

        self.inner
            .0
            .create_response(
                self.interaction_id,
                self.interaction_token(),
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: data.build().into(),
                },
            )
            .await?;
        Ok(())
    }

    #[inline]
    pub async fn defer(&self) -> UnitRespondResult {
        self.defer_as(false).await
    }

    #[inline]
    pub async fn defer_ephem(&self) -> UnitRespondResult {
        self.defer_as(true).await
    }

    pub async fn update_followup(
        &self,
        message_id: Id<MessageMarker>,
        content: &str,
    ) -> UnitFollowupResult {
        self.inner
            .0
            .update_followup(self.interaction_token(), message_id)
            .content(Some(content))
            .await?;
        Ok(())
    }
}

pub trait AcknowledgementAware {
    type FollowupError;
    type RespondError;
    type RespondOrFollowupError: From<Self::RespondError> + From<Self::FollowupError>;

    fn acknowledged(&self) -> bool;
    async fn respond(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError>;
    async fn respond_ephemeral(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError>;
    async fn update(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError>;
    async fn followup(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::FollowupError>;
    async fn followup_ephemeral(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::FollowupError>;

    async fn respond_or_update(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError> {
        if self.acknowledged() {
            return self.update(&content.into()).await;
        }
        self.respond(content).await
    }
    async fn respond_or_followup(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondOrFollowupError> {
        if self.acknowledged() {
            return Ok(self.followup(&content.into()).await?);
        }

        Ok(self.respond(content).await?)
    }
    async fn respond_ephemeral_or_followup(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondOrFollowupError> {
        if self.acknowledged() {
            return Ok(self.followup_ephemeral(&content.into()).await?);
        }

        Ok(self.respond_ephemeral(content).await?)
    }
}

impl AcknowledgementAware for (Interface<'_>, bool) {
    type FollowupError = crate::error::core::FollowupError;
    type RespondError = twilight_http::Error;
    type RespondOrFollowupError = crate::error::core::FollowupError;

    fn acknowledged(&self) -> bool {
        self.1
    }

    async fn respond_ephemeral(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError> {
        self.0.respond_ephemeral(content).await
    }

    async fn respond(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError> {
        self.0.respond(content).await
    }

    async fn update(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError> {
        self.0.update_no_components_embeds(content).await
    }

    async fn followup(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::FollowupError> {
        self.0.followup(content).await
    }

    async fn followup_ephemeral(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::FollowupError> {
        self.0.followup_ephemeral(content).await
    }
}

impl<'a> Client<'a> {
    pub const fn new(client: twilight_http::client::InteractionClient<'a>) -> Self {
        Self(client)
    }

    pub fn interfaces(self, interaction: &Interaction) -> Interface<'a> {
        Interface {
            inner: self,
            interaction_token: interaction.token.clone(),
            interaction_id: interaction.id,
        }
    }

    pub async fn register_global_commands(&self) -> Result<(), RegisterGlobalCommandsError> {
        let commands = self
            .0
            .set_global_commands(&[SLASH_COMMANDS.as_slice(), MESSAGE_COMMANDS.as_slice()].concat())
            .await?
            .models()
            .await?;

        POPULATED_COMMANDS_MAP.get_or_init(|| {
            commands
                .into_iter()
                .map(|c| (&*c.name.clone().leak(), c))
                .collect()
        });

        Ok(())
    }

    pub fn populated_command<T: CommandInfoAware>(
    ) -> &'static twilight_model::application::command::Command {
        let name = T::name();
        POPULATED_COMMANDS_MAP
            .get()
            .unwrap_or_else(|| panic!("`POPULATED_COMMANDS_MAP` is not yet populated"))
            .get(name)
            .unwrap_or_else(|| panic!("command not found: {name}"))
    }

    pub fn mention_command<T: CommandInfoAware>() -> MentionCommand {
        let cmd = Self::populated_command::<T>();

        let name = cmd.name.clone().into();
        let id = cmd
            .id
            .unwrap_or_else(|| panic!("`POPULATED_COMMANDS_MAP` is not yet populated"));
        MentionCommand::new(name, id)
    }

    #[inline]
    pub const fn create_followup(
        &'a self,
        interaction_token: &'a str,
    ) -> twilight_http::request::application::interaction::CreateFollowup<'a> {
        self.0.create_followup(interaction_token)
    }

    #[inline]
    pub const fn delete_followup(
        &'a self,
        interaction_token: &'a str,
        message_id: Id<MessageMarker>,
    ) -> twilight_http::request::application::interaction::DeleteFollowup<'a> {
        self.0.delete_followup(interaction_token, message_id)
    }
}

pub struct MentionCommand {
    name: Box<str>,
    id: Id<CommandMarker>,
}

impl MentionCommand {
    pub const fn new(name: Box<str>, id: Id<CommandMarker>) -> Self {
        Self { name, id }
    }
}

impl Display for MentionCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("</")?;
        f.write_str(&self.name)?;
        f.write_char(':')?;
        self.id.fmt(f)?;
        f.write_char('>')?;

        Ok(())
    }
}
