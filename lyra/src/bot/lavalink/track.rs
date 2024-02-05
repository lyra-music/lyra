use twilight_lavalink::model::{TrackEnd, TrackStart};

use crate::bot::{core::model::BotStateRef, error::lavalink::ProcessResult, lavalink::ClientAware};

pub(super) struct StartContext<'a> {
    inner: &'a twilight_lavalink::model::TrackStart,
    bot: BotStateRef<'a>,
}

impl crate::bot::core::model::BotState {
    pub(super) const fn as_track_start_context<'a>(
        &'a self,
        inner: &'a TrackStart,
    ) -> StartContext<'a> {
        StartContext { inner, bot: self }
    }
}

impl<'a> super::model::Process for StartContext<'a> {
    async fn process(self) -> ProcessResult {
        tracing::debug!("guild {} started {}", self.inner.guild_id, self.inner.track);
        // TODO: handle now playing message sending
        Ok(())
    }
}

pub(super) struct EndContext<'a> {
    inner: &'a twilight_lavalink::model::TrackEnd,
    bot: BotStateRef<'a>,
}

impl crate::bot::core::model::BotState {
    pub(super) const fn as_track_end_context<'a>(&'a self, inner: &'a TrackEnd) -> EndContext<'a> {
        EndContext { inner, bot: self }
    }
}

impl<'a> super::model::Process for EndContext<'a> {
    async fn process(self) -> ProcessResult {
        let guild_id = self.inner.guild_id;
        tracing::debug!("guild {} ended   {}", guild_id, self.inner.track);
        let lavalink = self.bot.lavalink();

        // TODO: handle now playing message deleting
        let connection = lavalink.connection(guild_id);
        let queue = connection.queue();
        if queue.advance_locked() {
            queue.advance_unlock();
        } else {
            drop(connection);
            let mut connection = lavalink.connection_mut(guild_id);
            let queue = connection.queue_mut();
            queue.advance();
            if let Some(item) = connection.downgrade().queue().current() {
                lavalink
                    .player(guild_id)
                    .await?
                    .send(twilight_lavalink::model::Play::from((
                        guild_id,
                        item.track().track.clone(),
                    )))?;
            }
        }

        Ok(())
    }
}
