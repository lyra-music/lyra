use std::pin::Pin;

use derive_builder::Builder;
use twilight_model::{
    channel::{
        Message,
        message::{AllowedMentions, Component, Embed, MessageFlags},
    },
    http::attachment::Attachment,
};

use crate::{
    core::model::response::Respond,
    error::{BuildError, core::RespondError},
};

#[derive(Builder)]
#[builder(
    name = "FollowupBuilder",
    setter(into, strip_option),
    pattern = "owned",
    build_fn(error = "BuildError")
)]
pub struct FollowupResponse<'a, T: Respond> {
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

impl<'a, T: Respond> FollowupBuilder<'a, T> {
    pub(super) fn new(inner: &'a T) -> Self {
        Self::default().inner(inner)
    }
}

impl<'a, T: Respond + Send + Sync> IntoFuture for FollowupBuilder<'a, T> {
    type Output = Result<twilight_http::Response<Message>, RespondError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

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
