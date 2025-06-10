use tokio::sync::oneshot;
use twilight_model::application::interaction::InteractionData;

use crate::{
    command::AutocompleteCtx,
    error::gateway::{ProcessError, ProcessResult},
};

impl super::Context {
    pub(super) async fn process_as_autocomplete(mut self) -> ProcessResult {
        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            unreachable!()
        };

        let name = data.name.clone().into();
        let (tx, _) = oneshot::channel::<()>();
        let Err(source) = AutocompleteCtx::from_partial_data(
            self.inner,
            &data,
            self.bot,
            self.latency,
            self.sender,
            tx,
        )
        .execute(*data)
        .await
        else {
            return Ok(());
        };

        Err(ProcessError::AutocompleteExecute { name, source })
    }
}
