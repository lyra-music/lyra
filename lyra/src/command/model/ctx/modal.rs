use std::marker::PhantomData;

use twilight_model::application::interaction::{InteractionData, modal::ModalInteractionData};

use crate::core::model::response::initial::{
    defer_update::RespondWithDeferUpdate, message::update::RespondWithUpdate,
    modal::RespondWithModal,
};

use super::{
    CmdInnerMarkerKind, CmdMarker, ComponentMarker, Ctx, CtxContext, CtxKind, GuildMarker,
};

pub trait ModalInnerMarkerKind {}

pub struct CmdInnerMarker;
impl ModalInnerMarkerKind for CmdInnerMarker {}
pub struct ComponentInnerMarker;
impl ModalInnerMarkerKind for ComponentInnerMarker {}

pub struct ModalMarker<T: ModalInnerMarkerKind>(PhantomData<fn(T) -> T>);
pub type CmdModalMarker = ModalMarker<CmdInnerMarker>;
pub type ComponentModalMarker = ModalMarker<ComponentInnerMarker>;

impl<T: ModalInnerMarkerKind> CtxKind for ModalMarker<T> {}
#[expect(unused)]
pub type ModalCtx = Ctx<CmdModalMarker>;
pub type GuildModalCtx = Ctx<CmdModalMarker, GuildMarker>;

pub trait RespondWithModalKind: CtxKind {}
impl<T: CmdInnerMarkerKind> RespondWithModalKind for CmdMarker<T> {}
impl RespondWithModalKind for ComponentMarker {}

impl<T: RespondWithModalKind, C: CtxContext> RespondWithModal for Ctx<T, C> {}

impl<C: CtxContext, S: ModalInnerMarkerKind> Ctx<ModalMarker<S>, C> {
    pub fn submit_data(&self) -> &ModalInteractionData {
        let Some(InteractionData::ModalSubmit(ref data)) = self.inner.data else {
            unreachable!()
        };
        data
    }
}

impl<C: CtxContext> RespondWithDeferUpdate for Ctx<ComponentModalMarker, C> {}
impl<C: CtxContext> RespondWithUpdate for Ctx<ComponentModalMarker, C> {}
