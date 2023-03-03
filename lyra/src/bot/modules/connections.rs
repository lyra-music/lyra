use twilight_lavalink::model::Destroy;
use twilight_model::gateway::payload::{incoming::MessageCreate, outgoing::UpdateVoiceState};

use crate::bot::lib::models::Context;

pub async fn join(ctx: Context) -> anyhow::Result<()> {
    let bot = ctx.bot();
    ctx.respond()
        .content("What's the channel ID you want me to join?")?
        .await?;

    let author_id = ctx.author().id;
    let msg = bot
        .standby
        .wait_for_message(*ctx.channel_id(), move |new_msg: &MessageCreate| {
            new_msg.author.id == author_id
        })
        .await?;
    let channel_id = msg.content.parse()?;
    let guild_id = ctx.guild_id().expect("known to be present");

    bot.sender.command(&UpdateVoiceState::new(
        guild_id,
        Some(channel_id),
        false,
        false,
    ))?;

    ctx.respond()
        .content(&format!("Joined <#{channel_id}>!"))?
        .await?;

    Ok(())
}

pub async fn leave(ctx: Context) -> anyhow::Result<()> {
    let (msg, bot) = (ctx.message(), ctx.bot());

    tracing::debug!(
        "leave command in channel {} by {}",
        msg.channel_id,
        msg.author.name
    );

    let guild_id = msg.guild_id.unwrap();
    let player = ctx.lavalink().player(guild_id).await.unwrap();
    player.send(Destroy::from(guild_id))?;
    bot.sender
        .command(&UpdateVoiceState::new(guild_id, None, false, false))?;

    ctx.respond().content("Left the channel")?.await?;

    Ok(())
}
