use crate::core::model::response::initial::message::create::RespondWithMessage;

use super::{
    CmdInnerMarkerKind, CmdMarker, ComponentMarker, Ctx, CtxContext, CtxKind, ModalMarker,
    modal::ModalInnerMarkerKind,
};

pub trait RespondWithMessageKind: CtxKind {}
impl<T: CmdInnerMarkerKind> RespondWithMessageKind for CmdMarker<T> {}
impl<M: ModalInnerMarkerKind> RespondWithMessageKind for ModalMarker<M> {}
impl RespondWithMessageKind for ComponentMarker {}

impl<T: RespondWithMessageKind, C: CtxContext> RespondWithMessage for Ctx<T, C> {}
