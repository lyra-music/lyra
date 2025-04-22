use twilight_model::{
    application::interaction::{InteractionData, modal::ModalInteractionData},
    channel::message::Component,
};

use super::{
    AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, GuildMarker, Kind, Location, UnitRespondResult,
};

pub struct Marker;
impl Kind for Marker {}
pub type Modal = Ctx<Marker>;
pub type Guild = Ctx<Marker, GuildMarker>;

pub trait RespondVia: Kind {}
impl<T: AppCtxKind> RespondVia for AppCtxMarker<T> {}
impl RespondVia for ComponentMarker {}

impl<U: Location> Ctx<Marker, U> {
    pub fn submit_data(&self) -> &ModalInteractionData {
        let Some(InteractionData::ModalSubmit(ref data)) = self.inner.data else {
            unreachable!()
        };
        data
    }
}

impl<T: RespondVia, U: Location> Ctx<T, U> {
    pub async fn modal(
        &mut self,
        custom_id: impl Into<String> + Send,
        title: impl Into<String> + Send,
        text_inputs: impl IntoIterator<Item = impl Into<Component>> + Send,
    ) -> UnitRespondResult {
        let response = self
            .interface()
            .await?
            .modal(custom_id, title, text_inputs)
            .await;
        self.acknowledge();
        Ok(response?)
    }
}
