use twilight_model::http::interaction::InteractionResponseType;

use super::{EmptyResponseResult, Respond};

pub trait RespondWithDeferUpdate: Respond {
    async fn defer_update(&mut self) -> EmptyResponseResult {
        self.empty_acknowledge(InteractionResponseType::DeferredChannelMessageWithSource)
            .await
    }
}
