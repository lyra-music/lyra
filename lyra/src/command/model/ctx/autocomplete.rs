use crate::core::model::RespondAutocomplete;

use super::{Ctx, GuildMarker, Kind, Location};

pub struct Marker;
impl Kind for Marker {}
pub type Autocomplete = Ctx<Marker>;
#[allow(unused)]
pub type GuildAutocompleteCtx = Ctx<Marker, GuildMarker>;

impl<U: Location> RespondAutocomplete for Ctx<Marker, U> {}
