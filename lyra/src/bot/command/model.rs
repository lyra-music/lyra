mod ctx;

use std::sync::Arc;

use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption},
        InteractionDataResolved,
    },
    id::{
        marker::{CommandMarker, GenericMarker},
        Id,
    },
};

use crate::bot::error::{command::AutocompleteResult, CommandResult};

pub use self::ctx::{
    AutocompleteCtx, CommandDataAware, Ctx, CtxKind, MessageCtx, ModalCtx, RespondViaMessage,
    RespondViaModal, SlashCtx, UserCtx,
};

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
