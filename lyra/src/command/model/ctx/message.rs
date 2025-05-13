use twilight_model::{
    channel::message::{AllowedMentions, Component, Embed, MessageFlags},
    http::interaction::InteractionResponseData,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{core::model::MessageResponse, error::command::FollowupError};

use super::{
    AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, Kind, Location, ModalMarker, RespondResult,
    UnitRespondResult,
};

type MessageRespondResult = RespondResult<MessageResponse>;

pub trait RespondVia: Kind {}
impl<T: AppCtxKind> RespondVia for AppCtxMarker<T> {}
impl RespondVia for ModalMarker {}
impl RespondVia for ComponentMarker {}

impl<T: RespondVia, U: Location> Ctx<T, U> {
    fn base_response_data_builder() -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    async fn respond_with(
        &mut self,
        data: Option<InteractionResponseData>,
    ) -> MessageRespondResult {
        let response = self.interface().respond_with(data).await;
        self.acknowledge();
        response
    }

    pub async fn respond_embeds(
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

    pub async fn defer(&mut self) -> UnitRespondResult {
        self.acknowledge();
        self.interface().defer().await
    }
}

impl<T: RespondVia, U: Location> crate::core::model::AcknowledgementAware for Ctx<T, U> {
    type FollowupError = FollowupError;
    type RespondError = twilight_http::Error;
    type RespondOrFollowupError = crate::error::command::RespondOrFollowupError;

    fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    async fn respond(
        &mut self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError> {
        let data = Self::base_response_data_builder().content(content).build();
        self.respond_with(Some(data)).await
    }

    async fn respond_ephemeral(
        &mut self,
        content: impl Into<String> + Send,
    ) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .content(content)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_with(Some(data)).await
    }

    async fn update(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::RespondError> {
        self.interface().update_no_components_embeds(content).await
    }

    async fn followup(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::FollowupError> {
        Ok(self.interface().followup(content).await?)
    }

    async fn followup_ephemeral(
        &self,
        content: impl Into<String> + Send,
    ) -> Result<MessageResponse, Self::FollowupError> {
        Ok(self.interface().followup_ephemeral(content).await?)
    }
}
