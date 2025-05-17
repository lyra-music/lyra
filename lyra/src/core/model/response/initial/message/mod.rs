pub mod create;
pub mod update;

use std::pin::Pin;

use derive_builder::Builder;
use twilight_model::{
    channel::{
        Message,
        message::{AllowedMentions, Component, Embed, MessageFlags},
    },
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    },
};

use crate::{
    core::model::response::{EmptyResponse, Respond},
    error::{
        BuildError,
        core::{DeserialiseBodyFromHttpError, RespondError},
    },
};

enum InteractionResponseType2 {
    ChannelMessageWithSource,
    UpdateMessage,
}

impl From<InteractionResponseType2> for InteractionResponseType {
    fn from(value: InteractionResponseType2) -> Self {
        match value {
            InteractionResponseType2::ChannelMessageWithSource => Self::ChannelMessageWithSource,
            InteractionResponseType2::UpdateMessage => Self::UpdateMessage,
        }
    }
}

#[derive(Builder)]
#[builder(
    setter(into, strip_option),
    pattern = "owned",
    build_fn(error = "BuildError")
)]
pub struct Response<'a, T: Respond> {
    #[builder(private)]
    pub(in crate::core::model::response) inner: &'a mut T,
    #[builder(private)]
    interaction_response_type: InteractionResponseType2,
    /// Allowed mentions of the response.
    #[builder(default)]
    pub(crate) allowed_mentions: Option<AllowedMentions>,
    /// List of attachments on the response.
    #[builder(default)]
    pub(crate) attachments: Option<Vec<Attachment>>,
    /// List of components on the response.
    #[builder(default)]
    pub(crate) components: Option<Vec<Component>>,
    /// Content of the response.
    #[builder(default)]
    pub(crate) content: Option<String>,
    /// Embeds of the response.
    #[builder(default)]
    pub(crate) embeds: Option<Vec<Embed>>,
    /// Interaction response data flags.
    ///
    /// The supported flags are [`MessageFlags::SUPPRESS_EMBEDS`] and
    /// [`MessageFlags::EPHEMERAL`].
    #[builder(default)]
    pub(crate) flags: Option<MessageFlags>,
    /// Whether the response is TTS.
    #[builder(default)]
    pub(crate) tts: Option<bool>,
}

impl<'a, T: Respond> From<Response<'a, T>>
    for (&'a mut T, InteractionResponseType, InteractionResponseData)
{
    fn from(value: Response<'a, T>) -> Self {
        (
            value.inner,
            value.interaction_response_type.into(),
            InteractionResponseData {
                allowed_mentions: value.allowed_mentions,
                attachments: value.attachments,
                components: value.components,
                content: value.content,
                embeds: value.embeds,
                flags: value.flags,
                tts: value.tts,
                choices: None,
                custom_id: None,
                title: None,
            },
        )
    }
}

pub struct InitialResponseProxy<'a, T: Respond> {
    ctx: &'a T,
    #[expect(unused)]
    response: EmptyResponse,
}

impl<'a, T: Respond> InitialResponseProxy<'a, T> {
    pub(in crate::core::model::response) const fn new(ctx: &'a T, response: EmptyResponse) -> Self {
        Self { ctx, response }
    }
}

impl<T: Respond + Sync> InitialResponseProxy<'_, T> {
    pub async fn retrieve_response(
        self,
    ) -> Result<twilight_http::Response<Message>, twilight_http::Error> {
        self.ctx
            .interaction_client()
            .response(self.ctx.interaction_token())
            .await
    }

    pub async fn retrieve_message(self) -> Result<Message, DeserialiseBodyFromHttpError> {
        Ok(self.retrieve_response().await?.model().await?)
    }
}

impl<'a, T: Respond + Send> IntoFuture for ResponseBuilder<'a, T> {
    type Output = Result<InitialResponseProxy<'a, T>, RespondError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            match self.build() {
                Err(e) => Err(e.into()),
                Ok(resp) => {
                    let (ctx, kind, data) = resp.into();
                    let token = ctx.interaction_token();
                    let client = ctx.interaction_client();

                    let result = client
                        .create_response(
                            ctx.interaction_id(),
                            token,
                            &InteractionResponse {
                                kind,
                                data: Some(data),
                            },
                        )
                        .await
                        .map_err(RespondError::from);

                    if result.is_ok() {
                        ctx.acknowledge();
                    }
                    result.map(|r| InitialResponseProxy::new(ctx, r))
                }
            }
        })
    }
}
