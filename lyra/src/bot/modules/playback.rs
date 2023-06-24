use anyhow::Result;
use twilight_lavalink::model::{Pause, Seek, Stop};
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::bot::{
    commands::{models::App, Context},
    lavalink::Lavalinkful,
};

pub async fn pause(ctx: Context<App>) -> Result<()> {
    tracing::debug!(
        "pause command in channel {} by {}",
        ctx.channel_id(),
        ctx.author().name
    );

    let guild_id = ctx.guild_id().unwrap();
    let player = ctx.lavalink().player(guild_id).await.unwrap();
    let paused = player.paused();
    player.send(Pause::from((guild_id, !paused)))?;

    let action = if paused { "Unpaused " } else { "Paused" };

    ctx.respond(&format!("{action} the track")).await?;

    Ok(())
}

pub async fn seek(ctx: Context<App>) -> Result<()> {
    let bot = ctx.bot();
    let (author, channel_id) = (ctx.author(), ctx.channel_id());

    tracing::debug!("seek command in channel {} by {}", channel_id, author.name);
    ctx.http()
        .create_message(channel_id)
        .content("Where in the track do you want to seek to (in seconds)?")?
        .await?;

    let author_id = author.id;
    let msg = bot
        .standby()
        .wait_for_message(channel_id, move |new_msg: &MessageCreate| {
            new_msg.author.id == author_id
        })
        .await?;
    let guild_id = ctx.guild_id().unwrap();
    let position = msg.content.parse::<i64>()?;

    let player = ctx.lavalink().player(guild_id).await.unwrap();
    player.send(Seek::from((guild_id, position * 1000)))?;

    ctx.respond(&format!("Seeked to {position}s")).await?;

    Ok(())
}

pub async fn stop(ctx: Context<App>) -> Result<()> {
    tracing::debug!(
        "stop command in channel {} by {}",
        ctx.channel_id(),
        ctx.author().name
    );

    let guild_id = ctx.guild_id().unwrap();
    let player = ctx.lavalink().player(guild_id).await.unwrap();
    player.send(Stop::from(guild_id))?;

    ctx.respond("Stopped the track").await?;

    Ok(())
}
