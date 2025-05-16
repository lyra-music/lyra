use twilight_model::{
    application::command::CommandOptionChoice, http::interaction::InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{EmptyResponseResult, Respond};

pub trait RespondAutocomplete: Respond {
    async fn autocomplete(
        &mut self,
        choices: impl IntoIterator<Item = CommandOptionChoice>,
    ) -> EmptyResponseResult
    where
        Self: Sized,
    {
        let data = InteractionResponseDataBuilder::new().choices(choices);
        let kind = InteractionResponseType::ApplicationCommandAutocompleteResult;
        self.respond_and_acknowledge(kind, data.build()).await
    }
}
