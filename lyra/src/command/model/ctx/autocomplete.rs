use crate::core::model::response::initial::autocomplete::RespondAutocomplete;

use super::{Ctx, GuildMarker, Kind, Location};

pub struct Marker;
impl Kind for Marker {}
pub type Autocomplete = Ctx<Marker>;
#[expect(unused)]
pub type GuildAutocompleteCtx = Ctx<Marker, GuildMarker>;

impl<U: Location> RespondAutocomplete for Ctx<Marker, U> {}
