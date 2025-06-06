use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    LavalinkAware,
    command::{model::BotSlashCommand, require, util::is_message_at_bottom},
    component::config::now_playing::Toggle as ConfigNowPlayingToggle,
    core::{
        http::InteractionClient,
        model::{OwnedHttpAware, response::initial::message::create::RespondWithMessage},
    },
    gateway::GuildIdAware,
    lavalink::{DelegateMethods, NowPlayingData},
};

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "now-playing", desc = ".", contexts = "guild")]
pub enum NowPlaying {
    #[command(name = "bump")]
    Bump(Bump),
}

/// Bumps the now-playing track message to the bottom of the current text channel, deleting the old one.
#[derive(CommandModel, CreateCommand)]
#[command(name = "bump")]
pub struct Bump;

impl BotSlashCommand for Bump {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let player = require::player(&ctx)?;
        let data = player.data();
        let data_r = data.read().await;

        let track = require::current_track(data_r.queue())?;
        let Some(msg_id) = data_r.now_playing_message_id() else {
            ctx.note(format!(
                "Now-playing track messages sending are disabled in this server.\n\
                    -# Moderators can enable the feature by using {}.",
                InteractionClient::mention_command::<ConfigNowPlayingToggle>()
            ))
            .await?;
            return Ok(());
        };
        let channel_id = ctx.channel_id();
        if is_message_at_bottom(&ctx, channel_id, msg_id) {
            ctx.note(
                "The now-playing track message is already at the bottom of the current text channel.",
            ).await?;
        } else {
            ctx.out("🔽 Bumped the now-playing track message.").await?;
            let lava_data = ctx.lavalink().data();
            let guild_id = ctx.guild_id().into();
            let msg_data = NowPlayingData::new(&lava_data, guild_id, &data_r, track.track).await?;
            drop(data_r);

            let mut data_w = data.write().await;
            data_w.delete_now_playing_message(&ctx).await;
            let http = ctx.http_owned();
            data_w
                .new_now_playing_message_in(http, msg_data, channel_id)
                .await?;
            drop(data_w);
        }
        Ok(())
    }
}
