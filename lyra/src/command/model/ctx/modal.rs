use std::marker::PhantomData;

use twilight_model::application::interaction::{InteractionData, modal::ModalInteractionData};

use crate::core::model::{
    Followup, RespondAppCommandModal, RespondComponentModal, RespondWithDefer,
    RespondWithDeferUpdate, RespondWithModal, RespondWithUpdate,
};

use super::{AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, GuildMarker, Kind, Location};

pub trait ModalSrcMarker {}

pub struct AppCmdSrcMarker;
impl ModalSrcMarker for AppCmdSrcMarker {}
pub struct ComponentSrcMarker;
impl ModalSrcMarker for ComponentSrcMarker {}

pub struct Marker<T: ModalSrcMarker>(PhantomData<fn(T) -> T>);
pub type ModalFromAppCmd = Marker<AppCmdSrcMarker>;
pub type ModalFromComponent = Marker<ComponentSrcMarker>;

impl<T: ModalSrcMarker> Kind for Marker<T> {}
#[allow(unused)]
pub type Modal = Ctx<ModalFromAppCmd>;
pub type Guild = Ctx<ModalFromComponent, GuildMarker>;

pub trait RespondVia: Kind {}
impl<T: AppCtxKind> RespondVia for AppCtxMarker<T> {}
impl RespondVia for ComponentMarker {}

impl<T: RespondVia, U: Location> RespondWithModal for Ctx<T, U> {}

impl<U: Location, S: ModalSrcMarker> Ctx<Marker<S>, U> {
    pub fn submit_data(&self) -> &ModalInteractionData {
        let Some(InteractionData::ModalSubmit(ref data)) = self.inner.data else {
            unreachable!()
        };
        data
    }
}

impl<U: Location, M: ModalSrcMarker> Followup for Ctx<Marker<M>, U> {}
impl<U: Location, M: ModalSrcMarker> RespondWithDefer for Ctx<Marker<M>, U> {}

impl<U: Location> RespondAppCommandModal for Ctx<ModalFromAppCmd, U> {}

impl<U: Location> RespondWithDeferUpdate for Ctx<ModalFromComponent, U> {}
impl<U: Location> RespondWithUpdate for Ctx<ModalFromComponent, U> {}
impl<U: Location> RespondComponentModal for Ctx<ModalFromComponent, U> {}
