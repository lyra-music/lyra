use std::sync::Arc;

use twilight_gateway::{Latency, MessageSender};
use twilight_model::{
    application::interaction::application_command::{
        CommandData, CommandDataOption, CommandOptionValue,
    },
    gateway::payload::incoming::InteractionCreate,
};

use crate::bot::{
    command::model::{PartialCommandData, PartialInteractionData},
    core::model::OwnedBotState,
};

use super::{autocomplete::AutocompleteMarker, AppCtxKind, AppCtxMarker, Ctx, CtxKind};

pub trait CommandDataAware: CtxKind {}
impl<T: AppCtxKind> CommandDataAware for AppCtxMarker<T> {}
impl CommandDataAware for AutocompleteMarker {}

impl<T: CommandDataAware> Ctx<T> {
    pub fn from_partial_data(
        inner: Box<InteractionCreate>,
        data: &CommandData,
        bot: OwnedBotState,
        latency: Latency,
        sender: MessageSender,
    ) -> Self {
        Self {
            data: Some(PartialInteractionData::Command(PartialCommandData::new(
                data,
            ))),
            inner,
            bot,
            latency,
            sender,
            acknowledged: false,
            kind: std::marker::PhantomData::<fn(T) -> T>,
        }
    }

    pub fn command_data(&self) -> &PartialCommandData {
        let PartialInteractionData::Command(data) = self.interaction_data() else {
            unreachable!()
        };
        data
    }

    pub fn take_partial_command_data(&mut self) -> Option<PartialCommandData> {
        self.data.take().and_then(|d| match d {
            PartialInteractionData::Command(data) => Some(data),
            _ => None,
        })
    }

    pub fn command_name_full(&self) -> Box<str> {
        fn recurse_through_names(
            mut names: Vec<Arc<str>>,
            command_data_options: &[CommandDataOption],
        ) -> Vec<Arc<str>> {
            match command_data_options {
                [CommandDataOption {
                    name,
                    value:
                        CommandOptionValue::SubCommand(command_data_options)
                        | CommandOptionValue::SubCommandGroup(command_data_options),
                }] => {
                    names.push(name.clone().into());
                    recurse_through_names(names, command_data_options)
                }
                _ => names,
            }
        }

        recurse_through_names(
            vec![self.command_data().name.clone()],
            &self.command_data().options,
        )
        .join(" ")
        .into()
    }

    pub fn command_mention_full(&self) -> Box<str> {
        format!("</{}:{}>", self.command_name_full(), self.command_data().id).into()
    }
}
