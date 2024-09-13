use thiserror::Error;

use crate::command::model::PartialCommandData;

#[derive(Error, Debug)]
pub enum CommandExecuteError {
    #[error(transparent)]
    CheckUserAllowed(#[from] super::check::UserAllowedError),
    #[error(transparent)]
    InteractionParse(#[from] twilight_interactions::error::ParseError),
    #[error(transparent)]
    Command(#[from] super::Error),
    #[error("unknown command: {:?}", .0)]
    UnknownCommand(PartialCommandData),
}

pub enum FlattenedUntilUserNotAllowedCommandExecuteError {
    Sqlx,
    TaskJoin,
    UserNotAllowed,
    InteractionParse,
    UnknownCommand,
    Command,
}

pub use FlattenedUntilUserNotAllowedCommandExecuteError as Fuunacee;

impl CommandExecuteError {
    pub const fn flatten_until_user_not_allowed_as(&self) -> Fuunacee {
        match self {
            Self::CheckUserAllowed(e) => match e {
                super::check::UserAllowedError::AccessCalculatorBuild(e) => match e {
                    super::check::AccessCalculatorBuildError::Sqlx(_) => Fuunacee::Sqlx,
                    super::check::AccessCalculatorBuildError::TaskJoin(_) => Fuunacee::TaskJoin,
                },
                super::check::UserAllowedError::UserNotAllowed(_) => Fuunacee::UserNotAllowed,
            },
            Self::InteractionParse(_) => Fuunacee::InteractionParse,
            Self::UnknownCommand(_) => Fuunacee::UnknownCommand,
            Self::Command(_) => Fuunacee::Command,
        }
    }
}

#[derive(Error, Debug)]
pub enum AutocompleteExecuteError {
    #[error(transparent)]
    InteractionParse(#[from] twilight_interactions::error::ParseError),
    #[error(transparent)]
    Autocomplete(#[from] super::AutocompleteError),
    #[error("unknown autocomplete: {:?}", .0)]
    UnknownAutocomplete(PartialCommandData),
}
