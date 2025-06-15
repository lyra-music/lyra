use tokio::sync::oneshot;
use twilight_model::application::interaction::InteractionData;

use crate::{
    command::model::{AutocompleteCtx, GuildAutocompleteCtx},
    error::gateway::{ProcessError, ProcessResult},
};

impl super::Context {
    pub(super) async fn process_as_autocomplete(mut self) -> ProcessResult {
        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            unreachable!()
        };

        let name = data.name.clone().into();
        let (tx, _) = oneshot::channel::<()>();
        if self.inner.guild_id.is_some() {
            self.handle_guild_autocomplete(data, name, tx).await
        } else {
            self.handle_autocomplete(data, name, tx).await
        }
    }

    async fn handle_autocomplete(
        self,
        data: Box<twilight_model::application::interaction::application_command::CommandData>,
        name: Box<str>,
        tx: oneshot::Sender<()>,
    ) -> Result<(), ProcessError> {
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

    async fn handle_guild_autocomplete(
        self,
        data: Box<twilight_model::application::interaction::application_command::CommandData>,
        name: Box<str>,
        tx: oneshot::Sender<()>,
    ) -> Result<(), ProcessError> {
        let Err(source) = GuildAutocompleteCtx::from_partial_data(
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
