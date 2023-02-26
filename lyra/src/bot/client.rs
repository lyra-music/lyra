use std::{env, future::Future, net::SocketAddr, str::FromStr, sync::Arc};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::client::ClientBuilder;
use twilight_lavalink::Lavalink;
use twilight_model::channel::message::AllowedMentions;

use super::lib::models::{Context, StateRef};
use super::modules::connections::{join, leave};
use super::modules::playback::{pause, seek, stop};
use super::modules::queue::play;
use super::modules::tuning::volume;

fn spawn(fut: impl Future<Output = anyhow::Result<()>> + Send + 'static) {
    tokio::spawn(async move {
        if let Err(why) = fut.await {
            tracing::debug!("handler error: {why:?}");
        }
    });
}

pub async fn run() -> anyhow::Result<()> {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let (mut shard, state) = {
        let token = env::var("BOT_TOKEN")?;
        let lavalink_host = SocketAddr::from_str(&env::var("LAVALINK_HOST")?)?;
        let lavalink_auth = env::var("LAVALINK_AUTH")?;
        let shard_count = 1u64;

        let http = ClientBuilder::default()
            .default_allowed_mentions(AllowedMentions::default())
            .token(token.clone())
            .build();
        let user_id = http.current_user().await?.model().await?.id;

        let lavalink = Lavalink::new(user_id, shard_count);
        lavalink.add(lavalink_host, lavalink_auth).await?;

        let intents =
            Intents::GUILD_MESSAGES | Intents::GUILD_VOICE_STATES | Intents::MESSAGE_CONTENT;
        let shard = Shard::new(ShardId::ONE, token, intents);
        let sender = shard.sender();

        (shard, Arc::new(StateRef::new(http, lavalink, sender)))
    };

    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                tracing::warn!(?source, "error receiving event");

                if source.is_fatal() {
                    break;
                }

                continue;
            }
        };

        state.standby.process(&event);
        state.lavalink.process(&event).await?;

        if let Event::MessageCreate(msg) = event {
            if msg.guild_id.is_none() || !msg.content.starts_with('!') {
                continue;
            }

            let ctx = Context::new(msg.clone().0, Arc::clone(&state));

            match msg.content.split_whitespace().next() {
                Some("!join") => spawn(join(ctx)),
                Some("!leave") => spawn(leave(ctx)),
                Some("!pause") => spawn(pause(ctx)),
                Some("!play") => spawn(play(ctx)),
                Some("!seek") => spawn(seek(ctx)),
                Some("!stop") => spawn(stop(ctx)),
                Some("!volume") => spawn(volume(ctx)),
                _ => continue,
            }
        }
    }

    Ok(())
}
