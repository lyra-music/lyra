use thiserror::Error;

use crate::bot::command::model::PartialCommandData;

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

pub enum FlattenedUntilUserNotAllowedCommandExecuteError<'a> {
    Sqlx(&'a sqlx::Error),
    TaskJoin(&'a tokio::task::JoinError),
    UserNotAllowed(&'a crate::bot::error::UserNotAllowed),
    InteractionParse(&'a twilight_interactions::error::ParseError),
    UnknownCommand(&'a PartialCommandData),
    Command(&'a super::Error),
}

pub use FlattenedUntilUserNotAllowedCommandExecuteError as Fuunacee;

impl CommandExecuteError {
    pub const fn flatten_until_user_not_allowed_as(&self) -> Fuunacee {
        match self {
            Self::CheckUserAllowed(e) => match e {
                super::check::UserAllowedError::AccessCalculatorBuild(e) => match e {
                    super::check::AccessCalculatorBuildError::Sqlx(e) => Fuunacee::Sqlx(e),
                    super::check::AccessCalculatorBuildError::TaskJoin(e) => Fuunacee::TaskJoin(e),
                },
                super::check::UserAllowedError::UserNotAllowed(e) => Fuunacee::UserNotAllowed(e),
            },
            Self::InteractionParse(e) => Fuunacee::InteractionParse(e),
            Self::UnknownCommand(c) => Fuunacee::UnknownCommand(c),
            Self::Command(e) => Fuunacee::Command(e),
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
