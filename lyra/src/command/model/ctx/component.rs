use tokio::sync::oneshot;
use twilight_gateway::{Latency, MessageSender};
use twilight_model::{
    application::interaction::message_component::MessageComponentInteractionData, channel::Message,
    gateway::payload::incoming::InteractionCreate,
};

use crate::{
    command::model::PartialInteractionData,
    core::{
        model::{
            OwnedBotState,
            response::initial::{
                defer_update::RespondWithDeferUpdate, message::update::RespondWithUpdate,
            },
        },
        r#static::component::NowPlayingButtonType,
    },
};

use super::{ComponentMarker, Ctx, Location};

impl<U: Location> Ctx<ComponentMarker, U> {
    pub const fn from_data(
        inner: Box<InteractionCreate>,
        data: Box<MessageComponentInteractionData>,
        bot: OwnedBotState,
        latency: Latency,
        sender: MessageSender,
        acknowledgement: oneshot::Sender<()>,
    ) -> Self {
        Self {
            inner,
            bot,
            latency,
            sender,
            acknowledgement: Some(acknowledgement),
            data: Some(PartialInteractionData::Component(data)),
            acknowledged: false,
            kind: std::marker::PhantomData,
            location: std::marker::PhantomData,
        }
    }

    pub fn component_data_mut(&mut self) -> &mut MessageComponentInteractionData {
        let Some(PartialInteractionData::Component(data)) = self.data.as_mut() else {
            unreachable!()
        };
        data
    }

    pub fn message(&self) -> &Message {
        self.inner
            .message
            .as_ref()
            .expect("component contexts must have a message attached to the component")
    }

    pub fn take_custom_id_into_now_playing_button_type(&mut self) -> Option<NowPlayingButtonType> {
        let id = std::mem::take(&mut self.component_data_mut().custom_id);
        NowPlayingButtonType::try_from(id.as_str()).ok()
    }
}

impl<U: Location> RespondWithDeferUpdate for Ctx<ComponentMarker, U> {}
impl<U: Location> RespondWithUpdate for Ctx<ComponentMarker, U> {}
