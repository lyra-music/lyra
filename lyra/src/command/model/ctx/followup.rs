use crate::core::model::response::followup::Followup;

use super::{
    AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, Kind, Location,
    modal::{Marker, ModalSrcMarker},
};

pub trait FollowupCtxKind: Kind {}
impl<T: AppCtxKind> FollowupCtxKind for AppCtxMarker<T> {}
impl<M: ModalSrcMarker> FollowupCtxKind for Marker<M> {}
impl FollowupCtxKind for ComponentMarker {}

impl<T: FollowupCtxKind, U: Location> Followup for Ctx<T, U> {}
