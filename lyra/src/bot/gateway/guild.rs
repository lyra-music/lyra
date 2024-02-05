use twilight_model::gateway::payload::incoming::{GuildCreate, GuildDelete};

use super::model::Process;
use crate::bot::{
    core::model::{BotState, BotStateRef},
    error::gateway::ProcessResult,
};

pub(super) struct CreateContext<'a> {
    inner: &'a GuildCreate,
    bot: BotStateRef<'a>,
}

impl CreateContext<'_> {
    async fn increment_guild_count(&self) -> Result<(), sqlx::Error> {
        // FIXME: wait until twilight stop deserializing missing `Guild::unavailable` to false
        if !self.inner.unavailable {
            return Ok(());
        }

        sqlx::query!(
            r"--sql
            INSERT INTO guild_configs
                (id)
            SELECT $1
            WHERE
                NOT EXISTS (
                    SELECT 1 FROM guild_configs WHERE id = $1
                );",
            self.inner.id.get() as i64
        )
        .execute(self.bot.db())
        .await?;

        self.bot.info().increment_guild_count();
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
    inner: &'a GuildDelete,
    bot: BotStateRef<'a>,
}

impl BotState {
    pub(super) const fn as_guild_create_context<'a>(
        &'a self,
        inner: &'a GuildCreate,
    ) -> CreateContext {
        CreateContext { inner, bot: self }
    }

    pub(super) const fn as_guild_delete_context<'a>(
        &'a self,
        inner: &'a GuildDelete,
    ) -> DeleteContext {
        DeleteContext { inner, bot: self }
    }
}

impl DeleteContext<'_> {
    fn decrement_guild_count(&self) {
        self.bot.info().decrement_guild_count();
    }
}

impl Process for DeleteContext<'_> {
    async fn process(self) -> ProcessResult {
        self.decrement_guild_count();

        Ok(())
    }
}
