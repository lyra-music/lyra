use twilight_model::{channel::message::MessageFlags, http::interaction::InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::core::model::response::{EmptyResponseResult, Respond};

pub trait RespondWithDefer: Respond {
    async fn raw_defer(&mut self, ephemeral: bool) -> EmptyResponseResult {
        let mut data = InteractionResponseDataBuilder::new();
        if ephemeral {
            data = data.flags(MessageFlags::EPHEMERAL);
        }
        let kind = InteractionResponseType::DeferredChannelMessageWithSource;
        self.respond_and_acknowledge(kind, data.build()).await
    }
    #[inline]
    async fn defer(&mut self) -> EmptyResponseResult {
        self.raw_defer(false).await
    }
    #[inline]
    #[expect(unused)]
    async fn defer_ephemeral(&mut self) -> EmptyResponseResult {
        self.raw_defer(true).await
    }
}
