use anyhow::Result;
use async_trait::async_trait;
use twilight_model::gateway::payload::incoming::{GuildCreate, GuildDelete};

use super::{models::Process, ContextedLyra};

pub(super) trait GuildEvents {}

impl GuildEvents for Box<GuildCreate> {}
impl GuildEvents for GuildDelete {}

impl Context<'_, Box<GuildCreate>> {
    async fn increment_guild_count(&self) -> Result<()> {
        // FIXME: wait until twilight stop deserializing missing `Guild::unavailable` to false
        if !self.inner.unavailable {
            return Ok(());
        }

        sqlx::query!(
            r#"--sql
            INSERT INTO guild_configs
                (id)
            SELECT $1
            WHERE
                NOT EXISTS (
                    SELECT 1 FROM guild_configs WHERE id = $1
                );"#,
            self.inner.id.get() as i64
        )
        .execute(self.bot.db())
        .await?;

        self.bot.info().increment_guild_count();
        Ok(())
    }
}

#[async_trait]
impl Process for Context<'_, Box<GuildCreate>> {
    async fn process(self) -> Result<()> {
        self.increment_guild_count().await?;

        Ok(())
    }
}

impl Context<'_, GuildDelete> {
    async fn decrement_guild_count(&self) -> Result<()> {
        self.bot.info().decrement_guild_count();
        Ok(())
    }
}

#[async_trait]
impl Process for Context<'_, GuildDelete> {
    async fn process(self) -> Result<()> {
        self.decrement_guild_count().await?;

        Ok(())
    }
}

pub(super) struct Context<'a, T: GuildEvents> {
    inner: &'a T,
    bot: &'a ContextedLyra,
}

impl<'a, T: GuildEvents> Context<'a, T> {
    pub(super) const fn from_guild_events(event: &'a T, bot: &'a ContextedLyra) -> Self {
        Self { inner: event, bot }
    }
}
