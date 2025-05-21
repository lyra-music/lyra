use crate::core::model::response::followup::Followup;

use super::{
    CmdInnerMarkerKind, CmdMarker, ComponentMarker, Ctx, CtxContext, CtxKind,
    modal::{ModalInnerMarkerKind, ModalMarker},
};

pub trait FollowupKind: CtxKind {}
impl<T: CmdInnerMarkerKind> FollowupKind for CmdMarker<T> {}
impl<M: ModalInnerMarkerKind> FollowupKind for ModalMarker<M> {}
impl FollowupKind for ComponentMarker {}

impl<T: FollowupKind, C: CtxContext> Followup for Ctx<T, C> {}
