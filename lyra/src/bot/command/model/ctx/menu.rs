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
        self.partial_command_data()
            .target_id
            .expect("T: TargetIdAware")
    }
}

impl UserCtx {
    #[inline]
    pub fn target_user_id(&self) -> Id<TwilightUserMarker> {
        self.target_id().cast()
    }

    pub fn target_user(&self) -> &User {
        self.partial_command_data()
            .resolved
            .as_ref()
            .expect("interaction type is application command")
            .users
            .get(&self.target_user_id())
            .expect("user should be resolved")
    }
}

impl MessageCtx {
    #[inline]
    pub fn target_message_id(&self) -> Id<TwilightMessageMarker> {
        self.target_id().cast()
    }

    pub fn target_message(&self) -> &Message {
        self.partial_command_data()
            .resolved
            .as_ref()
            .expect("interaction type is application command")
            .messages
            .get(&self.target_message_id())
            .expect("message should be resolved")
    }
}
