use std::pin::Pin;

use twilight_http::{Response, response::marker::EmptyBody};
use twilight_model::{
    channel::{
        Message,
        message::{AllowedMentions, Component, Embed, MessageFlags},
    },
    http::attachment::Attachment,
};

use crate::error::core::{DeserialiseBodyFromHttpError, RespondError, RespondOrFollowupError};

use super::{
    Respond,
    followup::Followup,
    initial::message::{InitialResponseProxy, ResponseBuilder, create::RespondWithMessage},
};

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
            InteractionResponse::Followup(resp) => Ok(resp.model().await?),
            InteractionResponse::Initial(resp) => Ok(InitialResponseProxy::new(self.ctx, resp)
                .retrieve_message()
                .await?),
        }
    }
}

impl<'a, T: Respond + Send + Sync> IntoFuture for EitherResponse<'a, T> {
    type Output = Result<ResponseProxy<'a, T>, RespondOrFollowupError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let resp = self.inner.build().map_err(RespondError::Builder)?;

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
                    .map_err(RespondError::from)?;

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
                fn [<$name _f>](&mut self, content: impl ::std::convert::Into<::std::string::String>) -> EitherResponse<'_, Self>
                where
                    Self: ::std::marker::Sized + ::std::marker::Send + $crate::core::model::response::initial::message::create::RespondWithMessage,
                {
                    self.hid_f(format!("{} {}", $crate::core::r#const::exit_code::$emoji, content.into()))
                }
            }
        )+
    }
}

pub trait RespondOrFollowup: Respond + RespondWithMessage + Followup {
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

impl<T: Respond + RespondWithMessage + Followup> RespondOrFollowup for T {}
