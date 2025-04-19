use twilight_model::{
    application::interaction::InteractionDataResolved,
    channel::Message as TwilightMessage,
    id::{
        Id,
        marker::{
            GenericMarker, MessageMarker as TwilightMessageMarker, UserMarker as TwilightUserMarker,
        },
    },
    user::User as TwilightUser,
};

use super::{AppCtxKind, AppCtxMarker, Ctx, Location};

pub struct UserAppMarker;
impl AppCtxKind for UserAppMarker {}
pub type UserMarker = AppCtxMarker<UserAppMarker>;
pub type User = Ctx<UserMarker>;

pub struct MessageAppMarker;
impl AppCtxKind for MessageAppMarker {}
pub type MessageMarker = AppCtxMarker<MessageAppMarker>;
pub type Message = Ctx<MessageMarker>;

pub trait TargetIdAware: AppCtxKind {}
impl TargetIdAware for UserAppMarker {}
impl TargetIdAware for MessageAppMarker {}

impl<T: TargetIdAware + AppCtxKind, U: Location> Ctx<AppCtxMarker<T>, U> {
    pub const fn target_id(&self) -> Id<GenericMarker> {
        self.command_data()
            .target_id
            .expect("target-id-aware contexts must have a target id")
    }

    const fn resolved_data(&self) -> &InteractionDataResolved {
        self.command_data()
            .resolved
            .as_ref()
            .expect("target-id-aware contexts must have a resolved mention resources data")
    }
}

impl<U: Location> Ctx<UserMarker, U> {
    #[inline]
    pub const fn target_user_id(&self) -> Id<TwilightUserMarker> {
        self.target_id().cast()
    }

    pub fn target_user(&self) -> &TwilightUser {
        self.resolved_data()
            .users
            .get(&self.target_user_id())
            .expect("user contexts must contain a resolve mention of the target user id")
    }
}

impl<U: Location> Ctx<MessageMarker, U> {
    #[inline]
    pub const fn target_message_id(&self) -> Id<TwilightMessageMarker> {
        self.target_id().cast()
    }

    pub fn target_message(&self) -> &TwilightMessage {
        self.resolved_data()
            .messages
            .get(&self.target_message_id())
            .expect("message contexts must contain a resolve mention of the target message id")
    }
}
