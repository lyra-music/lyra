use twilight_http::{request::application::interaction::UpdateResponse, Response};
use twilight_model::{
    application::{command::CommandOptionChoice, interaction::Interaction},
    channel::{
        message::{component::ActionRow, AllowedMentions, Embed, MessageFlags},
        Message,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{InteractionMarker, MessageMarker},
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::{
    command::{
        declare::{message_commands, slash_commands, POPULATED_COMMANDS_MAP},
        model::CommandInfoAware,
    },
    error::core::{FollowupResult, RegisterGlobalCommandsError, RespondResult},
};

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

    pub async fn update_no_components_embeds(&self, content: &str) -> MessageFollowupResult {
        Ok(self
            .update()
            .components(None)
            .embeds(None)
            .content(Some(content))
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
            .0
            .create_followup(self.interaction_token())
            .content(content)
            .await?)
    }

    pub async fn followup_ephem(&self, content: &str) -> MessageFollowupResult {
        Ok(self
            .inner
            .0
            .create_followup(self.interaction_token())
            .flags(MessageFlags::EPHEMERAL)
            .content(content)
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

    pub async fn delete_followup(&self, message_id: Id<MessageMarker>) -> UnitRespondResult {
        self.inner
            .0
            .delete_followup(self.interaction_token(), message_id)
            .await?;
        Ok(())
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
            .set_global_commands(
                &[slash_commands().as_slice(), message_commands().as_slice()].concat(),
            )
            .await?
            .models()
            .await?;

        POPULATED_COMMANDS_MAP.get_or_init(|| {
            commands
                .into_iter()
                .map(|c| (c.name.clone().into(), c))
                .collect()
        });

        Ok(())
    }

    pub fn populated_command<T: CommandInfoAware>(
    ) -> &'static twilight_model::application::command::Command {
        POPULATED_COMMANDS_MAP
            .get()
            .unwrap_or_else(|| panic!("`POPULATED_COMMANDS_MAP` is not yet populated"))
            .get(T::name())
            .unwrap_or_else(|| panic!("command not found: {}", T::name()))
    }

    pub fn mention_command<T: CommandInfoAware>() -> Box<str> {
        let cmd = Self::populated_command::<T>();

        let name = &cmd.name;
        let id = cmd
            .id
            .unwrap_or_else(|| panic!("`POPULATED_COMMANDS_MAP` is not yet populated"));
        format!("</{name}:{id}>").into_boxed_str()
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

pub type MessageResponse = Response<Message>;
pub type UnitRespondResult = RespondResult<()>;
pub type MessageRespondResult = RespondResult<MessageResponse>;
pub type UnitFollowupResult = FollowupResult<()>;
pub type MessageFollowupResult = FollowupResult<MessageResponse>;
