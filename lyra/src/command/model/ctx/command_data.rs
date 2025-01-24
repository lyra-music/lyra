use std::{hint::unreachable_unchecked, marker::PhantomData, sync::Arc};

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

use super::{autocomplete::Marker, AppCtxKind, AppCtxMarker, Ctx, Kind, Location};

pub trait Aware: Kind {}
impl<T: AppCtxKind> Aware for AppCtxMarker<T> {}
impl Aware for Marker {}

impl<T: Aware> Ctx<T> {
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
            location: PhantomData,
        }
    }
}

impl<T: Aware, U: Location> Ctx<T, U> {
    pub const fn command_data(&self) -> &PartialCommandData {
        let Some(PartialInteractionData::Command(data)) = self.data.as_ref() else {
            // SAFETY: `self` is `Ctx<impl CommandDataAware, _>`,
            //         so `data` will always be `PartialInteractionData::Command(_)`
            unsafe { unreachable_unchecked() }
        };
        data
    }

    pub fn into_command_data(self) -> Box<PartialCommandData> {
        let Some(PartialInteractionData::Command(command_data)) = self.data else {
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
}
