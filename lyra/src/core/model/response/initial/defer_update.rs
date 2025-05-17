use twilight_model::http::interaction::InteractionResponseType;

use crate::core::model::response::{EmptyResponseResult, Respond};

#[expect(unused)]
pub trait RespondWithDeferUpdate: Respond {
    async fn defer_update(&mut self) -> EmptyResponseResult {
        self.empty_acknowledge(InteractionResponseType::DeferredChannelMessageWithSource)
            .await
    }
}
