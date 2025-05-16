use twilight_model::{channel::message::Component, http::interaction::InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{EmptyResponseResult, Respond};

pub trait RespondWithModal: Respond {
    async fn modal(
        &mut self,
        custom_id: impl Into<String>,
        title: impl Into<String>,
        components: impl IntoIterator<Item = Component>,
    ) -> EmptyResponseResult {
        let data = InteractionResponseDataBuilder::new()
            .custom_id(custom_id)
            .title(title)
            .components(components)
            .build();
        self.respond_and_acknowledge(InteractionResponseType::Modal, data)
            .await
    }
}
