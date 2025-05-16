use std::pin::Pin;

use derive_builder::Builder;
use twilight_model::{
    channel::{
        Message,
        message::{AllowedMentions, Component, Embed},
    },
    http::attachment::Attachment,
    id::{
        Id,
        marker::{AttachmentMarker, MessageMarker},
    },
};

use crate::{
    core::model::response::Respond,
    error::{BuildError, core::RespondError},
};

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

impl<'a, T: Respond> UpdateFollowupBuilder<'a, T> {
    pub(super) fn new(inner: &'a T, message_id: Id<MessageMarker>) -> Self {
        Self::default().inner(inner).message_id(message_id)
    }
}

impl<'a, T: Respond + Send + Sync> IntoFuture for UpdateFollowupBuilder<'a, T> {
    type Output = Result<twilight_http::Response<Message>, RespondError>;

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
