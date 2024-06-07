use std::{hint::unreachable_unchecked, marker::PhantomData, sync::Arc};

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

use super::{
    autocomplete::AutocompleteMarker, AppCtxKind, AppCtxMarker, Ctx, CtxKind, CtxLocation,
};

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
            kind: PhantomData::<fn(T) -> T>,
            location: PhantomData,
        }
    }
}

impl<T: CommandDataAware, U: CtxLocation> Ctx<T, U> {
    pub fn command_data(&self) -> &PartialCommandData {
        // SAFETY: `self` is `Ctx<impl CommandDataAware, _>`,
        //         so `self.data` is present
        let data = unsafe { self.data.as_ref().unwrap_unchecked() };
        let PartialInteractionData::Command(data) = data else {
            // SAFETY:
            unsafe { unreachable_unchecked() }
        };
        data
    }

    pub fn into_command_data(self) -> PartialCommandData {
        // SAFETY: `self` is `Ctx<impl CommandDataAware, _>`,
        //         so `self.data` is present
        let data = unsafe { self.data.unwrap_unchecked() };
        let PartialInteractionData::Command(command_data) = data else {
            // SAFETY: `self` is `Ctx<impl CommandDataAware, _>`,
            //         so `data` will always be `PartialInteractionData::Command(_)`
            unsafe { unreachable_unchecked() }
        };
        command_data
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
