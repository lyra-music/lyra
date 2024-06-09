mod ctx;

use std::sync::Arc;

use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption},
        Interaction, InteractionDataResolved,
    },
    channel::Channel,
    guild::{PartialMember, Permissions},
    id::{
        marker::{ChannelMarker, CommandMarker, GenericMarker, UserMarker},
        Id,
    },
    user::User,
};

use crate::error::{command::AutocompleteResult, CommandResult};

pub use self::ctx::{
    AutocompleteCtx, CommandDataAware, Ctx, CtxKind, GuildCtx, GuildModalCtx, MessageCtx,
    RespondViaMessage, RespondViaModal, SlashCtx, UserCtx, WeakGuildCtx,
};

pub trait NonPingInteraction {
    unsafe fn author_unchecked(&self) -> &User;
    unsafe fn author_id_unchecked(&self) -> Id<UserMarker> {
        // SAFETY: interaction type is not `Ping`, so an author exists
        let author = unsafe { self.author_unchecked() };
        author.id
    }
    unsafe fn channel_unchecked(&self) -> &Channel;
    unsafe fn channel_id_unchecked(&self) -> Id<ChannelMarker> {
        // SAFETY: interaction type is not `Ping`, so a channel exists
        let channel = unsafe { self.channel_unchecked() };
        channel.id
    }
}

impl NonPingInteraction for Interaction {
    unsafe fn author_unchecked(&self) -> &User {
        // SAFETY: interaction type is not `Ping`, so an author exists
        unsafe { self.author().unwrap_unchecked() }
    }

    unsafe fn channel_unchecked(&self) -> &Channel {
        // SAFETY: interaction type is not `Ping`, so channel exists
        unsafe { self.channel.as_ref().unwrap_unchecked() }
    }
}

pub trait GuildInteraction {
    unsafe fn member_unchecked(&self) -> &PartialMember;
    unsafe fn author_permissions_unchecked(&self) -> Permissions {
        // SAFETY: interaction invoked in a guild, so member exists
        let member = unsafe { self.member_unchecked() };
        // SAFETY: member was sent from an interaction, so permissions is sent
        unsafe { member.permissions.unwrap_unchecked() }
    }
}

impl GuildInteraction for Interaction {
    unsafe fn member_unchecked(&self) -> &PartialMember {
        // SAFETY: interaction invoekd in a guild, so member exists
        unsafe { self.member.as_ref().unwrap_unchecked() }
    }
}

#[derive(Debug)]
pub struct PartialCommandData {
    pub id: Id<CommandMarker>,
    pub name: Arc<str>,
    pub target_id: Option<Id<GenericMarker>>,
    pub resolved: Option<InteractionDataResolved>,
    pub options: Box<[CommandDataOption]>,
}

impl PartialCommandData {
    pub fn new(data: &CommandData) -> Self {
        Self {
            id: data.id,
            name: data.name.to_string().into(),
            target_id: data.target_id,
            resolved: data.resolved.clone(),
            options: data.options.clone().into(),
        }
    }
}

#[non_exhaustive]
pub enum PartialInteractionData {
    Command(PartialCommandData),
    _Other,
}

pub trait CommandInfoAware {
    fn name() -> &'static str;
}

pub trait BotSlashCommand: CommandInfoAware {
    async fn run(self, ctx: SlashCtx) -> CommandResult;
}

pub trait BotUserCommand: CommandInfoAware {
    async fn run(ctx: UserCtx) -> CommandResult;
}

pub trait BotMessageCommand: CommandInfoAware {
    async fn run(ctx: MessageCtx) -> CommandResult;
}

pub trait BotAutocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult;
}
