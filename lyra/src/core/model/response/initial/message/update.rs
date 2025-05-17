use crate::core::model::response::Respond;

use super::{InteractionResponseType2, ResponseBuilder};

pub trait RespondWithUpdate: Respond {
    fn update(&mut self) -> ResponseBuilder<'_, Self>
    where
        Self: Sized,
    {
        ResponseBuilder::default()
            .inner(self)
            .interaction_response_type(InteractionResponseType2::UpdateMessage)
    }
}
