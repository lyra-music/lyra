use std::{
    fmt::{Display, Write},
    sync::Arc,
};

use twilight_model::{
    application::{command::Command, interaction::Interaction},
    id::{Id, marker::CommandMarker},
};

use crate::{
    command::{declare::POPULATED_COMMAND_MAP, model::CommandStructureAware},
    error::core::SetGlobalCommandsError,
};

use super::{model::CtxHead, r#static::application};

pub struct Client(Arc<twilight_http::Client>);

impl Client {
    pub const fn new(client: Arc<twilight_http::Client>) -> Self {
        Self(client)
    }

    pub fn ctx(self, interaction: &Interaction) -> CtxHead {
        CtxHead::new(self.0, interaction.id, interaction.token.clone().into())
    }

    pub async fn set_global_commands(
        &self,
        commands: &[Command],
    ) -> Result<(), SetGlobalCommandsError> {
        let commands = self
            .0
            .interaction(application::id())
            .set_global_commands(commands)
            .await?
            .models()
            .await?;

        POPULATED_COMMAND_MAP.get_or_init(|| {
            commands
                .into_iter()
                .map(|c| (&*c.name.clone().leak(), c))
                .collect()
        });

        Ok(())
    }

    fn qualified_command_name<T: CommandStructureAware>() -> QualifiedCommandName<'static> {
        match (T::ROOT_NAME, T::PARENT_NAME, T::NAME) {
            // TODO: This code relies on the invaariant `root != inner != leaf` for it to function properly.
            // However, the Discord API does not enforce said invariant.
            // This is not a future-proof design, and should be revisited some time in the future.
            (root, None, leaf) if root == leaf => QualifiedCommandName::Root(root),
            (_, None, _) => {
                panic!("a slash command has a root different from its leaf yet has no parent")
            }
            (root, Some(inner), leaf) if root == inner => QualifiedCommandName::Group(root, leaf),
            (root, Some(inner), leaf) => QualifiedCommandName::SubGroup(root, inner, leaf),
        }
    }

    pub fn populated_command_root<T: CommandStructureAware>()
    -> &'static twilight_model::application::command::Command {
        let name = T::ROOT_NAME;
        POPULATED_COMMAND_MAP
            .get()
            .expect("populated command map must be populated")
            .get(name)
            .unwrap_or_else(|| panic!("command not found: {name}"))
    }

    pub fn mention_command<T: CommandStructureAware>() -> MentionCommand<'static> {
        let cmd = Self::populated_command_root::<T>();

        let id = cmd.id.expect("populated command map must be populated");
        MentionCommand::new(id, Self::qualified_command_name::<T>())
    }
}

pub enum QualifiedCommandName<'a> {
    Root(&'a str),
    Group(&'a str, &'a str),
    SubGroup(&'a str, &'a str, &'a str),
}

impl Display for QualifiedCommandName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedCommandName::Root(r) => f.write_str(r),
            QualifiedCommandName::Group(r, l) => {
                f.write_str(r)?;
                f.write_char(' ')?;
                f.write_str(l)
            }
            QualifiedCommandName::SubGroup(r, i, l) => {
                f.write_str(r)?;
                f.write_char(' ')?;
                f.write_str(i)?;
                f.write_char(' ')?;
                f.write_str(l)
            }
        }
    }
}

pub struct MentionCommand<'a> {
    id: Id<CommandMarker>,
    name: QualifiedCommandName<'a>,
}

impl<'a> MentionCommand<'a> {
    pub const fn new(id: Id<CommandMarker>, name: QualifiedCommandName<'a>) -> Self {
        Self { id, name }
    }
}

impl Display for MentionCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("</")?;
        self.name.fmt(f)?;
        f.write_char(':')?;
        self.id.fmt(f)?;
        f.write_char('>')?;

        Ok(())
    }
}
