use twilight_model::{
    application::interaction::{modal::ModalInteractionData, InteractionData},
    channel::message::Component,
};

use super::{AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, CtxKind, UnitRespondResult};

pub struct ModalMarker;
impl CtxKind for ModalMarker {}
pub type ModalCtx = Ctx<ModalMarker>;

pub trait RespondViaModal: CtxKind {}
impl<T: AppCtxKind> RespondViaModal for AppCtxMarker<T> {}
impl RespondViaModal for ComponentMarker {}

impl Ctx<ModalMarker> {
    pub fn submit_data(&self) -> &ModalInteractionData {
        let Some(InteractionData::ModalSubmit(ref data)) = self.inner.data else {
            unreachable!()
        };
        data
    }
}

impl<T: RespondViaModal> Ctx<T> {
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
