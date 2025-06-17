use std::sync::Arc;

use twilight_http::{Client, client::InteractionClient};
use twilight_model::id::{Id, marker::InteractionMarker};

use crate::core::statik::application;

use super::response::{
    Respond,
    followup::Followup,
    initial::{
        defer::RespondWithDefer,
        message::{create::RespondWithMessage, update::RespondWithUpdate},
        modal::RespondWithModal,
    },
};

pub struct CtxHead {
    acknowledged: bool,
    interaction_id: Id<InteractionMarker>,
    interaction_token: Box<str>,
    client: Arc<Client>,
}

impl CtxHead {
    pub const fn new(
        client: Arc<Client>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: Box<str>,
    ) -> Self {
        Self {
            client,
            interaction_id,
            interaction_token,
            acknowledged: false,
        }
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }
}

impl Respond for CtxHead {
    fn is_acknowledged(&self) -> bool {
        self.acknowledged
    }

    fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    fn interaction_id(&self) -> Id<InteractionMarker> {
        self.interaction_id
    }

    fn interaction_token(&self) -> &str {
        &self.interaction_token
    }

    fn interaction_client(&self) -> InteractionClient<'_> {
        self.client.interaction(application::id())
    }
}

impl Followup for CtxHead {}
impl RespondWithMessage for CtxHead {}
impl RespondWithDefer for CtxHead {}
impl RespondWithModal for CtxHead {}

impl RespondWithUpdate for CtxHead {}
