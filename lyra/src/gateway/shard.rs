use lyra_ext::num::u64_to_i64_truncating;
use tokio::task::JoinSet;
use twilight_gateway::ShardId;
use twilight_model::gateway::payload::incoming::Ready;

use crate::{
    command::declare,
    core::{
        model::{BotState, BotStateRef, DatabaseAware},
        r#static::application,
    },
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
    ) -> ReadyContext<'a> {
        ReadyContext {
            inner,
            shard_id,
            bot: self,
        }
    }
}

impl Process for ReadyContext<'_> {
    async fn process(self) -> ProcessResult {
        application::set_id(self.inner.application.id);
        let commands = declare::commands();
        tracing::info!("registering {} global command(s)", commands.len());
        self.bot
            .interaction()
            .set_global_commands(&commands)
            .await?;

        let guild_count = self.inner.guilds.len();
        tracing::info!("running in {guild_count} guild(s)");
        self.bot
            .info()
            .reset_guild_count(self.shard_id, guild_count);

        let mut set = JoinSet::new();

        self.inner.guilds.iter().for_each(|g| {
            let db = self.bot.db().clone();
            let guild_id = u64_to_i64_truncating(g.id.get());
            set.spawn(async move {
                sqlx::query!(
                    "INSERT INTO guild_configs
                        (id)
                    SELECT $1
                    WHERE
                        NOT EXISTS (
                            SELECT 1 FROM guild_configs WHERE id = $1
                        );",
                    guild_id
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
