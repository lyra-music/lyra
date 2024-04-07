use twilight_model::{
    channel::Message,
    id::{
        marker::{
            GenericMarker, MessageMarker as TwilightMessageMarker, UserMarker as TwilightUserMarker,
        },
        Id,
    },
    user::User,
};

use super::{AppCtxKind, AppCtxMarker, Ctx};

pub struct UserAppMarker;
impl AppCtxKind for UserAppMarker {}
pub type UserMarker = AppCtxMarker<UserAppMarker>;
pub type UserCtx = Ctx<UserMarker>;

pub struct MessageAppMarker;
impl AppCtxKind for MessageAppMarker {}
pub type MessageMarker = AppCtxMarker<MessageAppMarker>;
pub type MessageCtx = Ctx<UserMarker>;

pub trait TargetIdAware: AppCtxKind {}
impl TargetIdAware for UserAppMarker {}
impl TargetIdAware for MessageAppMarker {}

impl<T: TargetIdAware + AppCtxKind> Ctx<AppCtxMarker<T>> {
    pub fn target_id(&self) -> Id<GenericMarker> {
        self.command_data()
            .target_id
            .expect("`self.command_data().target_id` must exist")
    }
}

impl UserCtx {
    #[inline]
    pub fn target_user_id(&self) -> Id<TwilightUserMarker> {
        self.target_id().cast()
    }

    pub fn target_user(&self) -> &User {
        self.command_data()
            .resolved
            .as_ref()
            .expect("`self.command_data().resolved` must exist")
            .users
            .get(&self.target_user_id())
            .expect("user must exist")
    }
}

impl MessageCtx {
    #[inline]
    pub fn target_message_id(&self) -> Id<TwilightMessageMarker> {
        self.target_id().cast()
    }

    pub fn target_message(&self) -> &Message {
        self.command_data()
            .resolved
            .as_ref()
            .expect("`self.command_data().resolved` must exist")
            .messages
            .get(&self.target_message_id())
            .expect("message must exist")
    }
}
