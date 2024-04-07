use tokio::task::JoinSet;
use twilight_gateway::ShardId;
use twilight_model::gateway::payload::incoming::Ready;

use crate::bot::{
    core::model::{BotState, BotStateRef},
    error::gateway::ProcessResult,
};

use super::model::Process;

pub(super) struct ReadyContext<'a> {
    inner: &'a Ready,
    shard_id: ShardId,
    bot: BotStateRef<'a>,
}

impl BotState {
    pub(super) const fn as_ready_context<'a>(
        &'a self,
        inner: &'a Ready,
        shard_id: ShardId,
    ) -> ReadyContext {
        ReadyContext {
            inner,
            shard_id,
            bot: self,
        }
    }
}

impl Process for ReadyContext<'_> {
    async fn process(self) -> ProcessResult {
        let guild_count = self.inner.guilds.len();
        tracing::info!("running in {guild_count} guild(s)");
        self.bot
            .info()
            .reset_guild_count(self.shard_id, guild_count);

        let mut set = JoinSet::new();

        self.inner.guilds.iter().for_each(|g| {
            let db = self.bot.db().clone();
            let g = g.id.get() as i64;
            set.spawn(async move {
                sqlx::query!(
                    r"--sql
                    INSERT INTO guild_configs
                        (id)
                    SELECT $1
                    WHERE
                        NOT EXISTS (
                            SELECT 1 FROM guild_configs WHERE id = $1
                        );",
                    g
                )
                .execute(&db)
                .await?;

                Ok::<_, sqlx::Error>(())
            });
        });

        while set.join_next().await.is_some() {}
        Ok(())
    }
}
