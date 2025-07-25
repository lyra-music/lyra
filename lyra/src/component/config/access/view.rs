use lyra_proc::view_access_ids;
use twilight_interactions::command::{CommandModel, CreateCommand};

use twilight_util::builder::embed::EmbedBuilder;

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    core::{
        konst::colour::EMBED_DEFAULT,
        model::{DatabaseAware, response::initial::message::create::RespondWithMessage},
    },
    error::CommandResult,
};

/// Views the currently configured access controls for channels, roles and members.
#[derive(CommandModel, CreateCommand)]
#[command(name = "view")]
pub struct View;

impl BotGuildSlashCommand for View {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let embed = EmbedBuilder::new()
            .title("🔐 Guild's Access Settings")
            .color(EMBED_DEFAULT);
        view_access_ids!(
            users,
            roles,
            threads,
            text_channels,
            voice_channels,
            category_channels
        );
        let embed = embed.validate()?.build();

        ctx.respond().embeds([embed]).await?;
        Ok(())
    }
}
