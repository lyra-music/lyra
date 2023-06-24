use anyhow::Result;
use async_trait::async_trait;
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
    commands::{
        models::{App, LyraCommand},
        Context,
    },
    ext::utils::EmptyStringMap,
    lib::consts::texts::EMPTY_EMBED_FIELD,
    modules::config::access::mode::AccessModePrettify,
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "view",
    desc = "Views the currently configured access controls for channels, roles and members"
)]
pub struct View;

#[async_trait]
impl LyraCommand for View {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        let guild_id = ctx.guild_id_unchecked().get() as i64;
        let db = ctx.db();

        let embed = EmbedBuilder::new().title("🔐 Guild's Access Settings");
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
