use std::{marker::PhantomData, sync::Arc};

use tokio::sync::oneshot;
use twilight_gateway::{Latency, MessageSender};
use twilight_model::{
    application::interaction::application_command::{
        CommandData, CommandDataOption, CommandOptionValue,
    },
    gateway::payload::incoming::InteractionCreate,
};

use crate::{
    command::model::{PartialCommandData, PartialInteractionData},
    core::model::OwnedBotState,
};

use super::{
    CmdInnerMarkerKind, CmdMarker, Ctx, CtxContext, CtxKind, autocomplete::AutocompleteMarker,
};

pub trait CommandDataAwareKind: CtxKind {}
impl<T: CmdInnerMarkerKind> CommandDataAwareKind for CmdMarker<T> {}
impl CommandDataAwareKind for AutocompleteMarker {}

impl<T: CommandDataAwareKind> Ctx<T> {
    pub fn from_partial_data(
        inner: Box<InteractionCreate>,
        data: &CommandData,
        bot: OwnedBotState,
        latency: Latency,
        sender: MessageSender,
        acknowledgement: oneshot::Sender<()>,
    ) -> Self {
        Self {
            data: Some(PartialInteractionData::Command(Box::new(
                PartialCommandData::new(data),
            ))),
            inner,
            bot,
            latency,
            sender,
            acknowledged: false,
            acknowledgement: Some(acknowledgement),
            kind: PhantomData::<fn(T) -> T>,
            context: PhantomData,
        }
    }
}

impl<T: CommandDataAwareKind, C: CtxContext> Ctx<T, C> {
    pub const fn command_data(&self) -> &PartialCommandData {
        let Some(PartialInteractionData::Command(data)) = self.data.as_ref() else {
            unreachable!()
        };
        data
    }

    pub fn into_command_data(self) -> Box<PartialCommandData> {
        let Some(PartialInteractionData::Command(command_data)) = self.data else {
            unreachable!()
        };
        command_data
    }

    pub fn command_name_full(&self) -> Box<str> {
        fn recurse_through_names(
            mut names: Vec<Arc<str>>,
            command_data_options: &[CommandDataOption],
        ) -> Vec<Arc<str>> {
            match command_data_options {
                [
                    CommandDataOption {
                        name,
                        value:
                            CommandOptionValue::SubCommand(command_data_options)
                            | CommandOptionValue::SubCommandGroup(command_data_options),
                    },
                ] => {
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
}
