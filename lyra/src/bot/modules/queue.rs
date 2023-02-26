use hyper::{Body, Request};
use twilight_lavalink::{http::LoadedTracks, model::Play};
use twilight_model::gateway::payload::incoming::MessageCreate;

use crate::bot::lib::models::Context;

pub async fn play(ctx: Context) -> anyhow::Result<()> {
    let state = ctx.state();
    let (author, channel_id) = (ctx.author(), *ctx.channel_id());

    tracing::debug!("play command in channel {} by {}", channel_id, author.name);
    ctx.respond()
        .content("What's the URL of the audio to play?")?
        .await?;

    let author_id = author.id;
    let msg = state
        .standby
        .wait_for_message(channel_id, move |new_msg: &MessageCreate| {
            new_msg.author.id == author_id
        })
        .await?;
    let guild_id = msg.guild_id.unwrap();

    let player = ctx.lavalink().player(guild_id).await.unwrap();
    let (parts, body) = twilight_lavalink::http::load_track(
        player.node().config().address,
        &msg.content,
        &player.node().config().authorization,
    )?
    .into_parts();
    let req = Request::from_parts(parts, Body::from(body));
    let res = state.hyper.request(req).await?;
    let response_bytes = hyper::body::to_bytes(res.into_body()).await?;

    let loaded = serde_json::from_slice::<LoadedTracks>(&response_bytes)?;

    if let Some(track) = loaded.tracks.first() {
        player.send(Play::from((guild_id, &track.track)))?;

        let content = format!(
            "Playing **{:?}** by **{:?}**",
            track.info.title, track.info.author
        );
        ctx.respond().content(&content)?.await?;
    } else {
        ctx.respond().content("Didn't find any results")?.await?;
    }

    Ok(())
}
