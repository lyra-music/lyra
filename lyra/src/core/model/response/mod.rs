use twilight_http::{Response, client::InteractionClient, response::marker::EmptyBody};
use twilight_model::{
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{Id, marker::InteractionMarker},
};

pub mod either;
pub mod followup;
pub mod initial;

pub type EmptyResponse = Response<EmptyBody>;
pub type EmptyResponseResult = Result<EmptyResponse, twilight_http::Error>;

pub trait Respond {
    fn is_acknowledged(&self) -> bool;
    fn acknowledge(&mut self);
    fn interaction_id(&self) -> Id<InteractionMarker>;
    fn interaction_token(&self) -> &str;
    fn interaction_client(&self) -> InteractionClient<'_>;

    #[expect(async_fn_in_trait)]
    async fn raw_respond_and_acknowledge(
        &mut self,
        kind: InteractionResponseType,
        data: Option<InteractionResponseData>,
    ) -> EmptyResponseResult {
        let resp = self
            .interaction_client()
            .create_response(
                self.interaction_id(),
                self.interaction_token(),
                &InteractionResponse { kind, data },
            )
            .await;
        if resp.is_ok() {
            self.acknowledge();
        }
        resp
    }
    #[inline]
    #[expect(async_fn_in_trait)]
    async fn respond_and_acknowledge(
        &mut self,
        kind: InteractionResponseType,
        data: InteractionResponseData,
    ) -> EmptyResponseResult {
        self.raw_respond_and_acknowledge(kind, Some(data)).await
    }
    #[inline]
    #[expect(async_fn_in_trait)]
    async fn empty_acknowledge(&mut self, kind: InteractionResponseType) -> EmptyResponseResult {
        self.raw_respond_and_acknowledge(kind, None).await
    }
}
