use twilight_model::{
    application::interaction::{modal::ModalInteractionData, InteractionData},
    channel::message::Component,
};

use super::{
    AppCtxKind, AppCtxMarker, ComponentMarker, Ctx, CtxKind, CtxLocation, Guild, UnitRespondResult,
};

pub struct ModalMarker;
impl CtxKind for ModalMarker {}
pub type ModalCtx = Ctx<ModalMarker>;
pub type GuildModalCtx = Ctx<ModalMarker, Guild>;

pub trait RespondViaModal: CtxKind {}
impl<T: AppCtxKind> RespondViaModal for AppCtxMarker<T> {}
impl RespondViaModal for ComponentMarker {}

impl<U: CtxLocation> Ctx<ModalMarker, U> {
    pub fn submit_data(&self) -> &ModalInteractionData {
        let Some(InteractionData::ModalSubmit(ref data)) = self.inner.data else {
            // SAFETY: `self` is `Ctx<ModalMarker, _>`,
            //         so `self.inner.data` will always be `InteractionData::ModalSubmit(_)`
            unsafe { std::hint::unreachable_unchecked() }
        };
        data
    }
}

impl<T: RespondViaModal, U: CtxLocation> Ctx<T, U> {
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
