use twilight_model::application::interaction::InteractionData;

use crate::error::gateway::ProcessResult;

impl super::Context {
    #[expect(clippy::unused_async)]
    pub(super) async fn process_as_modal(mut self) -> ProcessResult {
        let Some(InteractionData::ModalSubmit(data)) = self.inner.data.take() else {
            unreachable!()
        };
        tracing::trace!(?data);

        Ok(())
    }
}
