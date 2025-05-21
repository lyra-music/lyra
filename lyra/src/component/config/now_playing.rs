use lyra_ext::num::u64_to_i64_truncating;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    LavalinkAware,
    command::{SlashCmdCtx, model::BotSlashCommand, require},
    core::model::{
        DatabaseAware, OwnedHttpAware, response::initial::message::create::RespondWithMessage,
    },
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::{DelegateMethods, NowPlayingData},
};
use lyra_proc::BotCommandGroup;

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "now-playing", desc = ".")]
pub enum NowPlaying {
    #[command(name = "toggle")]
    Toggle(Toggle),
}

/// Toggles whether now-playing track messages should be automatically sent or not.
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle")]
pub struct Toggle;

impl BotSlashCommand for Toggle {
    async fn run(self, ctx: SlashCmdCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let new_now_playing = sqlx::query!(
            "UPDATE guild_configs SET now_playing = NOT now_playing WHERE id = $1 RETURNING now_playing;",
            u64_to_i64_truncating(ctx.guild_id().get()),
        )
        .fetch_one(ctx.db())
        .await?
        .now_playing;

        let maybe_data = require::player(&ctx).map(|p| p.data());
        let (emoji, action) = if new_now_playing {
            if let Ok(data) = maybe_data {
                let data_r = data.read().await;
                if let Ok(track) = require::current_track(data_r.queue()) {
                    let (c_data, gid) = (ctx.lavalink().data(), ctx.guild_id().into());
                    let np_data = NowPlayingData::new(&c_data, gid, &data_r, track.track).await?;
                    drop(data_r);
                    data.write()
                        .await
                        .new_now_playing_message_in(ctx.http_owned(), np_data, ctx.channel_id())
                        .await?;
                }
            }
            ("🔔", "Sending")
        } else {
            if let Ok(data) = maybe_data {
                if data.read().await.now_playing_message_id().is_some() {
                    data.write().await.delete_now_playing_message(&ctx).await;
                }
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
