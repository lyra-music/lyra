use twilight_model::application::command::CommandOptionChoice;

use super::{Ctx, CtxKind, UnitRespondResult};

pub struct AutocompleteMarker;
impl CtxKind for AutocompleteMarker {}
pub type AutocompleteCtx = Ctx<AutocompleteMarker>;

impl AutocompleteCtx {
    pub async fn autocomplete(
        &mut self,
        choices: impl IntoIterator<Item = CommandOptionChoice> + Send,
    ) -> UnitRespondResult {
        let response = self.interface().await?.autocomplete(choices).await;
        self.acknowledge();
        Ok(response?)
    }
}
