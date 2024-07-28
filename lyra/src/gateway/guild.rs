use lyra_ext::num::u64_to_i64_truncating;
use twilight_gateway::ShardId;
use twilight_model::gateway::payload::incoming::{GuildCreate, GuildDelete};

use super::model::Process;
use crate::{
    core::model::{BotState, BotStateRef},
    error::gateway::ProcessResult,
};

pub(super) struct CreateContext<'a> {
    inner: &'a GuildCreate,
    shard_id: ShardId,
    bot: BotStateRef<'a>,
}

impl CreateContext<'_> {
    async fn increment_guild_count(&self) -> Result<(), sqlx::Error> {
        /* FIXME: wait until twilight stop deserializing missing `Guild::unavailable` to false:
            https://github.com/twilight-rs/twilight/pull/2330
        */
        if !self.inner.unavailable {
            return Ok(());
        }

        sqlx::query!(
            "INSERT INTO guild_configs
                (id)
            SELECT $1
            WHERE
                NOT EXISTS (
                    SELECT 1 FROM guild_configs WHERE id = $1
                );",
            u64_to_i64_truncating(self.inner.id.get())
        )
        .execute(self.bot.db())
        .await?;

        self.bot.info().increment_guild_count(self.shard_id);
        Ok(())
    }
}

impl Process for CreateContext<'_> {
    async fn process(self) -> ProcessResult {
        self.increment_guild_count().await?;

        Ok(())
    }
}

pub(super) struct DeleteContext<'a> {
    _inner: &'a GuildDelete,
    shard_id: ShardId,
    bot: BotStateRef<'a>,
}

impl BotState {
    pub(super) const fn as_guild_create_context<'a>(
        &'a self,
        inner: &'a GuildCreate,
        shard_id: ShardId,
    ) -> CreateContext {
        CreateContext {
            inner,
            shard_id,
            bot: self,
        }
    }

    pub(super) const fn as_guild_delete_context<'a>(
        &'a self,
        inner: &'a GuildDelete,
        shard_id: ShardId,
    ) -> DeleteContext {
        DeleteContext {
            _inner: inner,
            shard_id,
            bot: self,
        }
    }
}

impl DeleteContext<'_> {
    fn decrement_guild_count(&self) {
        self.bot.info().decrement_guild_count(self.shard_id);
    }
}

impl Process for DeleteContext<'_> {
    async fn process(self) -> ProcessResult {
        self.decrement_guild_count();

        Ok(())
    }
}
