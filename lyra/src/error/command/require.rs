use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UnsuppressedError {
    Cache(#[from] crate::error::Cache),
    Suppressed(#[from] crate::error::Suppressed),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum InVoiceWithSomeoneElseError {
    Cache(#[from] crate::error::Cache),
    InVoiceWithoutSomeoneElse(#[from] crate::error::InVoiceWithoutSomeoneElse),
}
