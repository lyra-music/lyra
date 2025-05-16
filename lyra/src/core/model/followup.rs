use std::pin::Pin;

use derive_builder::Builder;
use twilight_http::{Response, response::marker::EmptyBody};
use twilight_model::{
    channel::{
        Message,
        message::{AllowedMentions, Component, Embed, MessageFlags},
    },
    http::attachment::Attachment,
    id::{
        Id,
        marker::{AttachmentMarker, MessageMarker},
    },
};

use crate::error::{BuildError, ResponseBuilderError, core::DeserialiseBodyFromHttpError};

use super::{
    RespondWithMessage,
    response::{EmptyResponseResult, Respond, ResponseBuilder},
};

#[derive(Builder)]
#[builder(
    setter(into, strip_option),
    pattern = "owned",
    build_fn(error = "BuildError")
)]
pub struct Followup<'a, T: Respond> {
    #[builder(private)]
    inner: &'a T,
    /// Allowed mentions of the response.
    #[builder(default)]
    allowed_mentions: Option<AllowedMentions>,
    /// List of attachments on the response.
    #[builder(default)]
    attachments: Option<Vec<Attachment>>,
    /// List of components on the response.
    #[builder(default)]
    components: Option<Vec<Component>>,
    /// Content of the response.
    #[builder(default)]
    content: Option<String>,
    /// Embeds of the response.
    #[builder(default)]
    embeds: Option<Vec<Embed>>,
    /// Interaction response data flags.
    ///
    /// The supported flags are [`MessageFlags::SUPPRESS_EMBEDS`] and
    /// [`MessageFlags::EPHEMERAL`].
    #[builder(default)]
    flags: Option<MessageFlags>,
    /// Whether the response is TTS.
    #[builder(default)]
    tts: Option<bool>,
}

impl<'a, T: Respond + Send> IntoFuture for FollowupBuilder<'a, T> {
    type Output = Result<twilight_http::Response<Message>, ResponseBuilderError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let resp = self.build()?;

            let ctx = resp.inner;
            let client = ctx.interaction_client();
            let mut f = client.create_followup(ctx.interaction_token());

            if let Some(ref content) = resp.content {
                f = f.content(content);
            }
            if let Some(flags) = resp.flags {
                f = f.flags(flags);
            }
            if let Some(ref embeds) = resp.embeds {
                f = f.embeds(embeds);
            }
            if let Some(ref components) = resp.components {
                f = f.components(components);
            }
            if let Some(ref attachments) = resp.attachments {
                f = f.attachments(attachments);
            }
            if let Some(ref allowed_mentions) = resp.allowed_mentions {
                f = f.allowed_mentions(Some(allowed_mentions));
            }
            if let Some(tts) = resp.tts {
                f = f.tts(tts);
            }

            Ok(f.await?)
        })
    }
}

#[derive(Builder)]
#[builder(
    setter(into, strip_option),
    pattern = "owned",
    build_fn(error = "BuildError")
)]
pub struct UpdateFollowup<'a, T: Respond> {
    #[builder(private)]
    inner: &'a T,
    #[builder(private)]
    message_id: Id<MessageMarker>,
    /// Allowed mentions of the response.
    #[builder(default)]
    allowed_mentions: Option<AllowedMentions>,
    /// List of attachments on the response.
    #[builder(default)]
    attachments: Option<Vec<Attachment>>,
    /// List of components on the response.
    #[builder(default)]
    components: Option<Option<Vec<Component>>>,
    /// Content of the response.
    #[builder(default)]
    content: Option<Option<String>>,
    /// Embeds of the response.
    #[builder(default)]
    embeds: Option<Option<Vec<Embed>>>,
    /// Specify multiple [`Id<AttachmentMarker>`]s already present in the target
    /// message to keep.
    ///
    /// If called, all unspecified attachments (except ones added with
    /// [`attachments`]) will be removed from the message. This is impossible if
    /// it would leave the message empty of `attachments`, `content`, or
    /// `embeds`. If not called, all attachments will be kept.
    ///
    /// [`attachments`]: Self::attachments
    #[builder(default)]
    keep_attachment_ids: Option<Vec<Id<AttachmentMarker>>>,
}

impl<'a, T: Respond + Send + Sync> IntoFuture for UpdateFollowupBuilder<'a, T> {
    type Output = Result<twilight_http::Response<Message>, ResponseBuilderError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let resp = self.build()?;

            let ctx = resp.inner;
            let client = ctx.interaction_client();
            let mut u = client.update_followup(ctx.interaction_token(), resp.message_id);

            if let Some(ref content) = resp.content {
                u = u.content(content.as_deref());
            }
            if let Some(ref embeds) = resp.embeds {
                u = u.embeds(embeds.as_deref());
            }
            if let Some(ref components) = resp.components {
                u = u.components(components.as_deref());
            }
            if let Some(ref attachments) = resp.attachments {
                u = u.attachments(&attachments);
            }
            if let Some(ref allowed_mentions) = resp.allowed_mentions {
                u = u.allowed_mentions(Some(allowed_mentions)); // None is ignored
            }
            if let Some(ref attachment_ids) = resp.keep_attachment_ids {
                u = u.keep_attachment_ids(&attachment_ids);
            }

            Ok(u.await?)
        })
    }
}

struct EitherResponse<'a, T: Respond> {
    inner: ResponseBuilder<'a, T>,
    is_followup: bool,
}

impl<'a, T: Respond + Send> EitherResponse<'a, T> {
    fn new(inner: ResponseBuilder<'a, T>, is_followup: bool) -> Self {
        Self { inner, is_followup }
    }

    fn flags(self, flags: impl Into<MessageFlags>) -> Self {
        Self {
            inner: self.inner.flags(flags),
            ..self
        }
    }

    fn content(self, content: impl Into<String>) -> Self {
        Self {
            inner: self.inner.content(content),
            ..self
        }
    }

    fn embeds(self, embeds: impl Into<Vec<Embed>>) -> Self {
        Self {
            inner: self.inner.embeds(embeds),
            ..self
        }
    }

    fn components(self, components: impl Into<Vec<Component>>) -> Self {
        Self {
            inner: self.inner.components(components),
            ..self
        }
    }

    fn attachments(self, attachments: impl Into<Vec<Attachment>>) -> Self {
        Self {
            inner: self.inner.attachments(attachments),
            ..self
        }
    }

    fn allowed_mentions(self, mentions: impl Into<AllowedMentions>) -> Self {
        Self {
            inner: self.inner.allowed_mentions(mentions),
            ..self
        }
    }

    fn tts(self, tts: bool) -> Self {
        Self {
            inner: self.inner.tts(tts),
            ..self
        }
    }
}

enum InteractionResponse {
    Initial(Response<EmptyBody>),
    Followup(Response<Message>),
}

struct ResponseProxy<'a, T: Respond> {
    ctx: &'a T,
    response: InteractionResponse,
}

impl<'a, T: Respond> ResponseProxy<'a, T> {
    pub const fn response(&self) -> &InteractionResponse {
        &self.response
    }

    pub async fn retrieve_message(self) -> Result<Message, DeserialiseBodyFromHttpError> {
        match self.response {
            InteractionResponse::Initial(_) => Ok(self
                .ctx
                .interaction_client()
                .response(&self.ctx.interaction_token())
                .await?
                .model()
                .await?),
            InteractionResponse::Followup(resp) => Ok(resp.model().await?),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RespondOrFollloupError {
    Respond(#[from] ResponseBuilderError),
    Followup(#[from] twilight_http::Error),
}

impl<'a, T: Respond + Send + Sync> IntoFuture for EitherResponse<'a, T> {
    type Output = Result<ResponseProxy<'a, T>, RespondOrFollloupError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let resp = self.inner.build().map_err(ResponseBuilderError::Builder)?;

            if self.is_followup {
                let ctx = resp.inner;

                let client = ctx.interaction_client();
                let mut f = client.create_followup(ctx.interaction_token());

                if let Some(ref content) = resp.content {
                    f = f.content(content);
                }
                if let Some(flags) = resp.flags {
                    f = f.flags(flags);
                }
                if let Some(ref embeds) = resp.embeds {
                    f = f.embeds(embeds);
                }
                if let Some(ref components) = resp.components {
                    f = f.components(components);
                }
                if let Some(ref attachments) = resp.attachments {
                    f = f.attachments(attachments);
                }
                if let Some(ref allowed_mentions) = resp.allowed_mentions {
                    f = f.allowed_mentions(Some(allowed_mentions));
                }
                if let Some(tts) = resp.tts {
                    f = f.tts(tts);
                }

                Ok(ResponseProxy {
                    ctx,
                    response: InteractionResponse::Followup(f.await?),
                })
            } else {
                let (ctx, kind, data) = resp.into();
                let token = ctx.interaction_token();
                let client = ctx.interaction_client();

                let result = client
                    .create_response(
                        ctx.interaction_id(),
                        &token,
                        &twilight_model::http::interaction::InteractionResponse {
                            kind,
                            data: Some(data),
                        },
                    )
                    .await
                    .map_err(ResponseBuilderError::from)?;

                ctx.acknowledge();
                Ok(ResponseProxy {
                    ctx,
                    response: InteractionResponse::Initial(result),
                })
            }
        })
    }
}

macro_rules! generate_hid_variants {
    ($($name: ident => $emoji: ident),+$(,)?) => {
        $(
            ::paste::paste! {
                #[inline]
                fn [<$name f>](&self, content: impl ::std::convert::Into<::std::string::String>) -> FollowupBuilder<'_, Self>
                where
                    Self: ::std::marker::Sized + $crate::core::model::response::RespondWithMessage,
                {
                    self.hidf(format!("{} {}", $crate::core::r#const::exit_code::$emoji, content.into()))
                }
            }

            ::paste::paste! {
                #[inline]
                fn [<$name _f>](&mut self, content: impl ::std::convert::Into<::std::string::String>) -> EitherResponse<'_, Self>
                where
                    Self: ::std::marker::Sized + ::std::marker::Send + $crate::core::model::response::RespondWithMessage,
                {
                    self.hid_f(format!("{} {}", $crate::core::r#const::exit_code::$emoji, content.into()))
                }
            }
        )+
    }
}

pub trait FollowupTrait: Respond {
    fn raw_followup(&self) -> FollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        FollowupBuilder::default().inner(self)
    }
    #[inline]
    fn outf(&self, content: impl Into<String>) -> FollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        self.raw_followup().content(content.into())
    }
    #[inline]
    fn hidf(&self, content: impl Into<String>) -> FollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        self.raw_followup()
            .flags(MessageFlags::EPHEMERAL)
            .content(content.into())
    }

    fn respond_or_followup(&mut self) -> EitherResponse<'_, Self>
    where
        Self: Sized + RespondWithMessage,
    {
        let is_followup = self.is_acknowledged();
        EitherResponse {
            is_followup,
            inner: self.respond(),
        }
    }
    #[inline]
    fn out_f(&mut self, content: impl Into<String>) -> EitherResponse<'_, Self>
    where
        Self: Sized + RespondWithMessage + Send,
    {
        self.respond_or_followup().content(content)
    }
    #[inline]
    fn hid_f(&mut self, content: impl Into<String>) -> EitherResponse<'_, Self>
    where
        Self: Sized + RespondWithMessage + Send,
    {
        self.respond_or_followup()
            .flags(MessageFlags::EPHEMERAL)
            .content(content)
    }

    #[inline]
    fn update_followup(
        &self,
        message_id: impl Into<Id<MessageMarker>>,
    ) -> UpdateFollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        UpdateFollowupBuilder::default()
            .inner(self)
            .message_id(message_id.into())
    }

    #[inline]
    async fn delete_followup(
        &self,
        message_id: impl Into<Id<MessageMarker>>,
    ) -> EmptyResponseResult {
        self.interaction_client()
            .delete_followup(&self.interaction_token(), message_id.into())
            .await
    }

    generate_hid_variants! {
        note => NOTICE,
        susp => DUBIOUS,
        warn => WARNING,
        wrng => INVALID,
        nope => PROHIBITED,
        blck => FORBIDDEN,
        erro => KNOWN_ERROR,
        unkn => UNKNOWN_ERROR
    }
}
