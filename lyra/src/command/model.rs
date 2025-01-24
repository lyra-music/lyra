mod ctx;

use std::sync::Arc;

use twilight_interactions::command::CreateCommand;
use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption},
        message_component::MessageComponentInteractionData,
        Interaction, InteractionDataResolved,
    },
    channel::Channel,
    guild::{PartialMember, Permissions},
    id::{
        marker::{ChannelMarker, GenericMarker, UserMarker},
        Id,
    },
    user::User as TwilightUser,
};

use crate::error::{command::AutocompleteResult, CommandResult};

pub use self::ctx::{
    Autocomplete as AutocompleteCtx, CommandDataAware, Component as ComponentCtx, Ctx,
    Guild as GuildCtx, GuildModal as GuildModalCtx, GuildRef as GuildCtxRef, Kind as CtxKind,
    Message as MessageCtx, RespondViaMessage, RespondViaModal, Slash as SlashCtx, User,
};

pub trait NonPingInteraction {
    unsafe fn author_unchecked(&self) -> &TwilightUser;
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
    unsafe fn author_unchecked(&self) -> &TwilightUser {
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
    pub name: Arc<str>,
    pub target_id: Option<Id<GenericMarker>>,
    pub resolved: Option<InteractionDataResolved>,
    pub options: Box<[CommandDataOption]>,
}

impl PartialCommandData {
    pub fn new(data: &CommandData) -> Self {
        Self {
            name: data.name.to_string().into(),
            target_id: data.target_id,
            resolved: data.resolved.clone(),
            options: data.options.clone().into(),
        }
    }
}

#[non_exhaustive]
pub enum PartialInteractionData {
    Command(Box<PartialCommandData>),
    Component(Box<MessageComponentInteractionData>),
}

pub trait BotSlashCommand: CreateCommand {
    async fn run(self, ctx: SlashCtx) -> CommandResult;
}

pub trait BotUserCommand: CreateCommand {
    async fn run(ctx: User) -> CommandResult;
}

pub trait BotMessageCommand: CreateCommand {
    async fn run(ctx: MessageCtx) -> CommandResult;
}

pub trait BotAutocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult;
}
