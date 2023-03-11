use twilight_lavalink::model::Volume;
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::bot::commands::models::Context;

pub async fn volume(ctx: Context) -> anyhow::Result<()> {
    let (author, channel_id) = (ctx.author(), ctx.channel_id());

    tracing::debug!(
        "volume command in channel {} by {}",
        channel_id,
        author.name
    );
    ctx.respond("What's the volume you want to set (0-1000, 100 being the default)?")
        .await?;

    let author_id = author.id;
    let msg = ctx
        .bot()
        .standby()
        .wait_for_message(channel_id, move |new_msg: &MessageCreate| {
            new_msg.author.id == author_id
        })
        .await?;
    let guild_id = msg.guild_id.unwrap();
    let volume = msg.content.parse::<i64>()?;

    if !(0..=1000).contains(&volume) {
        ctx.respond("That's more than 1000").await?;

        return Ok(());
    }

    let player = ctx.lavalink().player(guild_id).await.unwrap();
    player.send(Volume::from((guild_id, volume)))?;

    ctx.respond(&format!("Set the volume to {volume}")).await?;

    Ok(())
}
