use itertools::Itertools;
use lyra_proc::view_access_ids;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::id::{
    marker::{ChannelMarker, RoleMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};

use crate::bot::{
    command::{
        model::{BotSlashCommand, SlashCommand},
        Ctx,
    },
    component::config::access::mode::AccessModePrettify,
    core::r#const::{colours::EMBED_DEFAULT, text::EMPTY_EMBED_FIELD},
    error::command::Result as CommandResult,
    ext::util::OptionMap,
    gateway::ExpectedGuildIdAware,
};

/// Views the currently configured access controls for channels, roles and members
#[derive(CommandModel, CreateCommand)]
#[command(name = "view")]
pub struct View;

impl BotSlashCommand for View {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        let guild_id = ctx.guild_id().get() as i64;
        let db = ctx.db();

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

        ctx.respond_embeds_only([embed]).await?;
        Ok(())
    }
}
