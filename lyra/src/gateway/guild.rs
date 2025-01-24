use lyra_ext::num::u64_to_i64_truncating;
use twilight_gateway::ShardId;
use twilight_model::{
    gateway::payload::incoming::{GuildCreate, GuildDelete},
    guild::Guild,
};

use super::model::Process;
use crate::{
    core::model::{BotState, BotStateRef, DatabaseAware},
    error::gateway::ProcessResult,
};

pub(super) struct CreateContext<'a> {
    inner: &'a GuildCreate,
    shard_id: ShardId,
    bot: BotStateRef<'a>,
}

impl CreateContext<'_> {
    async fn increment_guild_count(&self) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO guild_configs
                (id)
            SELECT $1
            WHERE
                NOT EXISTS (
                    SELECT 1 FROM guild_configs WHERE id = $1
                );",
            u64_to_i64_truncating(self.inner.id().get())
        )
        .execute(self.bot.db())
        .await?;

        self.bot.info().increment_guild_count(self.shard_id);
        Ok(())
    }
}

impl Process for CreateContext<'_> {
    async fn process(self) -> ProcessResult {
        // these are all theoretical GuildCreate objects based on their availability:
        // * GuildCreate::Available(Guild { unavailable: None, .. })                current user joined a guild
        // * GuildCreate::Available(Guild { unavailable: Some(false), .. })         a guild became available
        // * GuildCreate::Available(Guild { unavailable: Some(true), .. })          (can't-happen: deserialised as #5)
        // * GuildCreate::Unavailable(UnavailableGuild { unavailable: false, .. })  (can't-happen: deserialised as #2)
        // * GuildCreate::Unavailable(UnavailableGuild { unavailable: true, .. })   current user joined an unavailable guild
        //
        // only variants #1 and #5 are of interest, so just early returning upon #2 is sufficient
        if let GuildCreate::Available(Guild {
            unavailable: Some(false),
            ..
        }) = self.inner
        {
            return Ok(());
        }

        self.increment_guild_count().await?;
        Ok(())
    }
}

pub(super) struct DeleteContext<'a> {
    inner: &'a GuildDelete,
    shard_id: ShardId,
    bot: BotStateRef<'a>,
}

impl BotState {
    pub(super) const fn as_guild_create_context<'a>(
        &'a self,
        inner: &'a GuildCreate,
        shard_id: ShardId,
    ) -> CreateContext<'a> {
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
    ) -> DeleteContext<'a> {
        DeleteContext {
            inner,
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
        // these are all theoretical GuildDelete objects based on their availability:
        // * GuildDelete { unavailable: None, .. }          current user left a guild
        // * GuildDelete { unavailable: Some(false), .. }   (undocumented: possibly akin to #1)
        // * GuildDelete { unavailable: Some(true), .. }    a guild became unavailable
        //
        // only variants #1 and possibly #2 are of interest, so just early returning upon #3 is sufficient
        if self.inner.unavailable == Some(true) {
            return Ok(());
        }

        self.decrement_guild_count();
        Ok(())
    }
}
