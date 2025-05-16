use twilight_http::{Response, client::InteractionClient, response::marker::EmptyBody};
use twilight_model::{
    channel::message::AllowedMentions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{Id, marker::InteractionMarker},
};
use twilight_util::builder::InteractionResponseDataBuilder;

mod autocomplete;
mod defer;
mod defer_update;
mod message;
mod modal;

pub use {
    autocomplete::RespondAutocomplete,
    defer::RespondWithDefer,
    defer_update::RespondWithDeferUpdate,
    message::{RespondWithMessage, RespondWithUpdate, ResponseBuilder},
    modal::RespondWithModal,
};

use super::followup::FollowupTrait;

pub type EmptyResponse = Response<EmptyBody>;
pub type EmptyResponseResult = Result<EmptyResponse, twilight_http::Error>;

pub trait Respond {
    fn is_acknowledged(&self) -> bool;
    fn acknowledge(&mut self);
    fn interaction_id(&self) -> Id<InteractionMarker>;
    fn interaction_token(&self) -> &str;
    fn interaction_client(&self) -> InteractionClient<'_>;

    fn base_response_data_builder() -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    async fn raw_respond_and_acknowledge(
        &mut self,
        kind: InteractionResponseType,
        data: Option<InteractionResponseData>,
    ) -> EmptyResponseResult {
        let resp = self
            .interaction_client()
            .create_response(
                self.interaction_id(),
                &self.interaction_token(),
                &InteractionResponse { kind, data },
            )
            .await;
        if resp.is_ok() {
            self.acknowledge();
        }
        resp
    }
    #[inline]
    async fn respond_and_acknowledge(
        &mut self,
        kind: InteractionResponseType,
        data: InteractionResponseData,
    ) -> EmptyResponseResult {
        self.raw_respond_and_acknowledge(kind, Some(data)).await
    }
    #[inline]
    async fn empty_acknowledge(&mut self, kind: InteractionResponseType) -> EmptyResponseResult {
        self.raw_respond_and_acknowledge(kind, None).await
    }
}

pub trait RespondComponent:
    RespondWithModal
    + RespondWithDeferUpdate
    + RespondWithDefer
    + RespondWithMessage
    + RespondWithUpdate
    + FollowupTrait
{
}

pub trait RespondAppCommandModal: RespondWithMessage + RespondWithDefer + FollowupTrait {}

pub trait RespondComponentModal:
    RespondWithMessage + RespondWithDefer + RespondWithDeferUpdate + RespondWithUpdate + FollowupTrait
{
}

pub trait RespondAppCommand:
    RespondWithModal + RespondWithDefer + RespondWithMessage + FollowupTrait
{
}
