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

use super::{CmdInnerMarkerKind, CmdMarker, Ctx, CtxContext, GuildMarker};

pub struct UserCmdInnerMarker;
impl CmdInnerMarkerKind for UserCmdInnerMarker {}
pub type UserCmdMarker = CmdMarker<UserCmdInnerMarker>;
pub type UserCmdCtx = Ctx<UserCmdMarker>;

pub struct MessageCmdInnerMarker;
impl CmdInnerMarkerKind for MessageCmdInnerMarker {}
pub type MessageCmdMarker = CmdMarker<MessageCmdInnerMarker>;
pub type MessageCmdCtx = Ctx<MessageCmdMarker>;
pub type GuildMessageCmdCtx = Ctx<MessageCmdMarker, GuildMarker>;

pub trait TargetIdAwareKind: CmdInnerMarkerKind {}
impl TargetIdAwareKind for UserCmdInnerMarker {}
impl TargetIdAwareKind for MessageCmdInnerMarker {}

impl<T: TargetIdAwareKind + CmdInnerMarkerKind, C: CtxContext> Ctx<CmdMarker<T>, C> {
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

#[expect(unused)]
impl<C: CtxContext> Ctx<UserCmdMarker, C> {
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

impl<C: CtxContext> Ctx<MessageCmdMarker, C> {
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
