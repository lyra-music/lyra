use crate::core::model::response::initial::defer::RespondWithDefer;

use super::{
    CmdInnerMarkerKind, CmdMarker, ComponentMarker, Ctx, CtxContext, CtxKind,
    modal::{ModalInnerMarkerKind, ModalMarker},
};

pub trait RespondWithDeferKind: CtxKind {}
impl<T: CmdInnerMarkerKind> RespondWithDeferKind for CmdMarker<T> {}
impl<M: ModalInnerMarkerKind> RespondWithDeferKind for ModalMarker<M> {}
impl RespondWithDeferKind for ComponentMarker {}

impl<T: RespondWithDeferKind, C: CtxContext> RespondWithDefer for Ctx<T, C> {}
