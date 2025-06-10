pub mod app_command;
pub mod autocomplete;
pub mod component;
pub mod modal;

use std::sync::Arc;

use twilight_gateway::{Latency, MessageSender};
use twilight_model::{
    application::interaction::InteractionType, gateway::payload::incoming::InteractionCreate,
};

use super::model::Process;
use crate::{
    core::model::{BotState, OwnedBotState},
    error::gateway::ProcessResult,
};

pub(super) struct Context {
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
}

impl BotState {
    pub(super) const fn into_interaction_create_context(
        self: Arc<Self>,
        inner: Box<InteractionCreate>,
        latency: Latency,
        sender: MessageSender,
    ) -> Context {
        Context {
            inner,
            bot: self,
            sender,
            latency,
        }
    }
}

impl Process for Context {
    async fn process(self) -> ProcessResult {
        match self.inner.kind {
            InteractionType::ApplicationCommand => self.process_as_app_command().await,
            InteractionType::ApplicationCommandAutocomplete => self.process_as_autocomplete().await,
            InteractionType::MessageComponent => self.process_as_component().await,
            InteractionType::ModalSubmit => self.process_as_modal().await,
            InteractionType::Ping => Ok(()), // ignored
            _ => unimplemented!(),
        }
    }
}
