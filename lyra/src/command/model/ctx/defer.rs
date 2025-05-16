use crate::core::model::response::initial::defer::RespondWithDefer;

use super::{
    AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, Kind, Location,
    modal::{Marker, ModalSrcMarker},
};

pub trait DeferCtxKind: Kind {}
impl<T: AppCtxKind> DeferCtxKind for AppCtxMarker<T> {}
impl<M: ModalSrcMarker> DeferCtxKind for Marker<M> {}
impl DeferCtxKind for ComponentMarker {}

impl<T: DeferCtxKind, U: Location> RespondWithDefer for Ctx<T, U> {}
