use twilight_model::{
    channel::message::{AllowedMentions, Component, Embed, MessageFlags},
    http::interaction::InteractionResponseData,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::{core::model::MessageResponse, error::command::FollowupError};

use super::{AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, CtxKind, ModalMarker, RespondResult};

type MessageRespondResult = RespondResult<MessageResponse>;
type MessageFollowupResult = Result<MessageResponse, FollowupError>;

pub trait RespondViaMessage: CtxKind {}
impl<T: AppCtxKind> RespondViaMessage for AppCtxMarker<T> {}
impl RespondViaMessage for ModalMarker {}
impl RespondViaMessage for ComponentMarker {}

impl<T: RespondViaMessage> Ctx<T> {
    fn base_response_data_builder() -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    async fn respond_with(
        &mut self,
        data: Option<InteractionResponseData>,
    ) -> MessageRespondResult {
        let response = self.interface().await?.respond_with(data).await;
        self.acknowledge();
        Ok(response?)
    }

    pub async fn respond(&mut self, content: impl Into<String> + Send) -> MessageRespondResult {
        let data = Self::base_response_data_builder().content(content).build();
        self.respond_with(Some(data)).await
    }

    pub async fn update_no_components_embeds(&mut self, content: &str) -> MessageFollowupResult {
        Ok(self
            .interface()
            .await?
            .update_no_components_embeds(content)
            .await?)
    }

    pub async fn respond_embeds_only(
        &mut self,
        embeds: impl IntoIterator<Item = Embed> + Send,
    ) -> MessageRespondResult {
        let data = Self::base_response_data_builder().embeds(embeds).build();
        self.respond_with(Some(data)).await
    }

    pub async fn respond_embeds_and_components(
        &mut self,
        embeds: impl IntoIterator<Item = Embed> + Send,
        components: impl IntoIterator<Item = Component> + Send,
    ) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .embeds(embeds)
            .components(components)
            .build();
        self.respond_with(Some(data)).await
    }

    pub async fn ephem(&mut self, content: impl Into<String> + Send) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .content(content)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_with(Some(data)).await
    }

    pub async fn followup(&self, content: &str) -> MessageFollowupResult {
        Ok(self.interface().await?.followup(content).await?)
    }

    pub async fn followup_ephem(&self, content: &str) -> MessageFollowupResult {
        Ok(self.interface().await?.followup_ephem(content).await?)
    }
}
