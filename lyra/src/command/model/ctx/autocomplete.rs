use crate::core::model::response::initial::autocomplete::RespondAutocomplete;

use super::{Ctx, CtxContext, CtxKind, GuildMarker};

pub struct AutocompleteMarker;
impl CtxKind for AutocompleteMarker {}
pub type AutocompleteCtx = Ctx<AutocompleteMarker>;
#[expect(unused)]
pub type GuildAutocompleteCtx = Ctx<AutocompleteMarker, GuildMarker>;

impl<C: CtxContext> RespondAutocomplete for Ctx<AutocompleteMarker, C> {}
