use twilight_model::application::command::CommandOptionChoice;

use super::{Ctx, GuildMarker, Kind, Location, UnitRespondResult};

pub struct Marker;
impl Kind for Marker {}
pub type Autocomplete = Ctx<Marker>;
pub type GuildAutocompleteCtx = Ctx<Marker, GuildMarker>;

impl<U: Location> Ctx<Marker, U> {
    pub async fn autocomplete(
        &mut self,
        choices: impl IntoIterator<Item = CommandOptionChoice> + Send,
    ) -> UnitRespondResult {
        let response = self.interface().await?.autocomplete(choices).await;
        self.acknowledge();
        Ok(response?)
    }
}
