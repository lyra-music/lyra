use twilight_gateway::{Latency, MessageSender};
use twilight_model::{
    application::interaction::message_component::MessageComponentInteractionData, channel::Message,
    gateway::payload::incoming::InteractionCreate,
};

use crate::{command::model::PartialInteractionData, core::model::OwnedBotState};

use super::{ComponentMarker, Ctx, Location};

impl<U: Location> Ctx<ComponentMarker, U> {
    pub const fn from_data(
        inner: Box<InteractionCreate>,
        data: Box<MessageComponentInteractionData>,
        bot: OwnedBotState,
        latency: Latency,
        sender: MessageSender,
    ) -> Self {
        Self {
            inner,
            bot,
            latency,
            sender,
            data: Some(PartialInteractionData::Component(data)),
            acknowledged: false,
            acknowledgement: None,
            kind: std::marker::PhantomData,
            location: std::marker::PhantomData,
        }
    }

    pub fn component_data_mut(&mut self) -> &mut MessageComponentInteractionData {
        let Some(PartialInteractionData::Component(data)) = self.data.as_mut() else {
            // SAFETY: `self` is `Ctx<ComponentMarker>`,
            //         so `data` will always be `PartialInteractionData::Component(_)`
            unsafe { std::hint::unreachable_unchecked() }
        };
        data
    }

    pub fn message(&self) -> &Message {
        // SAFETY: `self` is `Ctx<ComponentMarker>`, so `self.inner.message` exists
        unsafe { self.inner.message.as_ref().unwrap_unchecked() }
    }

    pub fn take_custom_id(&mut self) -> String {
        std::mem::take(&mut self.component_data_mut().custom_id)
    }
}