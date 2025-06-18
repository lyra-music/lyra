mod ctx;

use std::{error::Error, sync::Arc};

use twilight_interactions::command::CreateCommand;
use twilight_model::{
    application::interaction::{
        Interaction, InteractionDataResolved,
        application_command::{CommandData, CommandDataOption},
        message_component::MessageComponentInteractionData,
    },
    channel::Channel,
    guild::{PartialMember, Permissions},
    id::{
        Id,
        marker::{ChannelMarker, GenericMarker, UserMarker},
    },
    user::User as TwilightUser,
};

use crate::error::{CommandResult, command::AutocompleteResult};

pub use self::ctx::{
    AutocompleteCtx, CmdInnerMarkerKind, CmdMarker, CtxKind, FollowupKind, GuildAutocompleteCtx,
    GuildComponentCtx, GuildCtx, GuildMessageCmdCtx, GuildModalCtx, GuildSlashCmdCtx,
    MessageCmdCtx, RespondWithDeferKind, RespondWithMessageKind, SlashCmdCtx, UserCmdCtx,
};

pub trait NonPingInteraction {
    fn author_expected(&self) -> &TwilightUser;
    fn author_id_expected(&self) -> Id<UserMarker> {
        self.author_expected().id
    }
    fn channel_expected(&self) -> &Channel;
    fn channel_id_expected(&self) -> Id<ChannelMarker> {
        self.channel_expected().id
    }
}

impl NonPingInteraction for Interaction {
    fn author_expected(&self) -> &TwilightUser {
        self.author()
            .expect("non-ping interactions should have an author")
    }

    fn channel_expected(&self) -> &Channel {
        self.channel
            .as_ref()
            .expect("non-ping interactions should have a channel")
    }
}

pub trait GuildInteraction {
    fn member_expected(&self) -> &PartialMember;
    fn author_permissions_expected(&self) -> Permissions {
        self.member_expected()
            .permissions
            .expect("member object sent from an interaction should have permissions")
    }
}

impl GuildInteraction for Interaction {
    fn member_expected(&self) -> &PartialMember {
        self.member
            .as_ref()
            .expect("interactions invoked in a guild should have a member")
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

pub trait CommandStructureAware: CreateCommand {
    const ROOT_NAME: &'static str = Self::NAME;
    const PARENT_NAME: Option<&'static str> = None;
}

pub trait BotSlashCommand: CommandStructureAware {
    async fn run(self, ctx: SlashCmdCtx) -> CommandResult;
}

pub trait BotSlashCommand2: CommandStructureAware {
    type Error: Error;
    type ResidualError: Error;

    #[expect(unused)]
    async fn run(self, ctx: &mut SlashCmdCtx) -> Result<(), Self::Error>;

    #[expect(unused)]
    async fn handle_error(
        ctx: &mut SlashCmdCtx,
        error: Self::Error,
    ) -> Result<(), Self::ResidualError>;
}

pub trait BotGuildSlashCommand: CommandStructureAware {
    async fn run(self, ctx: GuildSlashCmdCtx) -> CommandResult;
}

#[expect(unused)]
pub trait BotUserCommand: CreateCommand {
    async fn run(ctx: UserCmdCtx) -> CommandResult;
}

#[expect(unused)]
pub trait BotMessageCommand: CreateCommand {
    async fn run(ctx: MessageCmdCtx) -> CommandResult;
}

pub trait BotGuildMessageCommand {
    async fn run(ctx: GuildMessageCmdCtx) -> CommandResult;
}

#[expect(unused)]
pub trait BotAutocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult;
}

pub trait BotGuildAutocomplete {
    async fn execute(self, ctx: GuildAutocompleteCtx) -> AutocompleteResult;
}
