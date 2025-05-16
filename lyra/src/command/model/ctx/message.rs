use crate::core::model::response::initial::message::create::RespondWithMessage;

use super::{
    AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, Kind, Location, ModalMarker,
    modal::ModalSrcMarker,
};

pub trait RespondVia: Kind {}
impl<T: AppCtxKind> RespondVia for AppCtxMarker<T> {}
impl<M: ModalSrcMarker> RespondVia for ModalMarker<M> {}
impl RespondVia for ComponentMarker {}

impl<T: RespondVia, U: Location> RespondWithMessage for Ctx<T, U> {}
