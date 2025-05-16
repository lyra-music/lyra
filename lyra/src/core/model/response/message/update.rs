use super::{InteractionResponseType2, Respond, ResponseBuilder};

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
