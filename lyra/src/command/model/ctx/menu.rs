use twilight_model::{
    application::interaction::InteractionDataResolved,
    channel::Message as TwilightMessage,
    id::{
        marker::{
            GenericMarker, MessageMarker as TwilightMessageMarker, UserMarker as TwilightUserMarker,
        },
        Id,
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
    pub fn target_id(&self) -> Id<GenericMarker> {
        // SAFETY: `self` is `Ctx<impl TargetIdAware, _>`,
        //         so `self.partial_command_data().target_id` is present
        unsafe { self.command_data().target_id.unwrap_unchecked() }
    }

    fn resolved_data(&self) -> &InteractionDataResolved {
        // SAFETY: `self` is `Ctx<impl TargetIdAware, _>`,
        //         so `self.partial_command_data().resolved` is present
        unsafe { self.command_data().resolved.as_ref().unwrap_unchecked() }
    }
}

impl<U: Location> Ctx<UserMarker, U> {
    #[inline]
    pub fn target_user_id(&self) -> Id<TwilightUserMarker> {
        self.target_id().cast()
    }

    pub fn target_user(&self) -> &TwilightUser {
        // SAFETY: `self` is `Ctx<UserMarker, _>`,
        //         so `self.resolved_data().users.get(&self.target_user_id())` is present
        unsafe {
            self.resolved_data()
                .users
                .get(&self.target_user_id())
                .unwrap_unchecked()
        }
    }
}

impl<U: Location> Ctx<MessageMarker, U> {
    #[inline]
    pub fn target_message_id(&self) -> Id<TwilightMessageMarker> {
        self.target_id().cast()
    }

    pub fn target_message(&self) -> &TwilightMessage {
        // SAFETY: `self` is `Ctx<MessageMarker, _>`,
        //         so `self.resolved_data().messages.get(&self.target_message_id())` is present
        unsafe {
            self.resolved_data()
                .messages
                .get(&self.target_message_id())
                .unwrap_unchecked()
        }
    }
}
