use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    LavalinkAware,
    command::{
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        require,
    },
    core::model::{
        DatabaseAware, OwnedHttpAware, response::initial::message::create::RespondWithMessage,
    },
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::{DelegateMethods, NowPlayingData},
};
use lyra_proc::BotGuildCommandGroup;

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "now-playing", desc = ".")]
pub enum NowPlaying {
    #[command(name = "toggle")]
    Toggle(Toggle),
}

/// Toggles whether now-playing track messages should be automatically sent or not.
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle")]
pub struct Toggle;

impl BotGuildSlashCommand for Toggle {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let new_now_playing = sqlx::query!(
            "UPDATE guild_configs SET now_playing = NOT now_playing WHERE id = $1 RETURNING now_playing;",
            ctx.guild_id().get().cast_signed(),
        )
        .fetch_one(ctx.db())
        .await?
        .now_playing;

        let maybe_data = require::player(&ctx).map(|p| p.data());
        let (emoji, action) = if new_now_playing {
            if let Ok(data) = maybe_data
                && let data_r = data.read().await
                && let Ok(track) = require::current_track(data_r.queue())
            {
                let (c_data, gid) = (ctx.lavalink().data(), ctx.guild_id().into());
                let np_data = NowPlayingData::new(&c_data, gid, &data_r, track.track).await?;
                drop(data_r);
                data.write()
                    .await
                    .new_now_playing_message_in(ctx.http_owned(), np_data, ctx.channel_id())
                    .await?;
            }
            ("🔔", "Sending")
        } else {
            if let Ok(data) = maybe_data
                && data.read().await.now_playing_message_id().is_some()
            {
                data.write().await.delete_now_playing_message().await;
            }
            ("🔕", "Not sending")
        };

        ctx.out(format!(
            "{emoji} **{action}** now-playing track messages from now on."
        ))
        .await?;
        Ok(())
    }
}
