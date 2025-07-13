use std::sync::OnceLock;

use tokio::sync::OnceCell;
use twilight_model::{
    guild::Emoji,
    id::{Id, marker::ApplicationMarker},
};

use crate::{core::model::HttpAware, error::core::DeserialiseBodyFromHttpError};

static ID: OnceLock<Id<ApplicationMarker>> = OnceLock::new();
static EMOJIS: OnceCell<&'static [Emoji]> = OnceCell::const_new();

pub fn set_id(id: Id<ApplicationMarker>) {
    ID.set(id).ok();
}

pub fn id() -> Id<ApplicationMarker> {
    *ID.get()
        .expect("ready event should have populated the application id")
}

pub async fn emojis(
    cx: &(impl HttpAware + Sync),
) -> Result<&'static [Emoji], DeserialiseBodyFromHttpError> {
    EMOJIS
        .get_or_try_init(|| async {
            let application_id = id();
            let req = cx.http().get_application_emojis(application_id);
            Ok(&*req.await?.model().await?.items.leak())
        })
        .await
        .copied()
}
