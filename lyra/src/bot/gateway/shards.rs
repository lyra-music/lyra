use anyhow::Result;
use async_trait::async_trait;
use tokio::task::JoinSet;
use twilight_model::gateway::payload::incoming::Ready;

use super::{models::Process, ContextedLyra};

pub(super) struct ReadyContext<'a> {
    inner: &'a Ready,
    bot: &'a ContextedLyra,
}

impl<'a> ReadyContext<'a> {
    pub(super) fn from_ready(event: &'a Ready, bot: &'a ContextedLyra) -> Self {
        Self { inner: event, bot }
    }
}

#[async_trait]
impl Process for ReadyContext<'_> {
    async fn process(self) -> Result<()> {
        let guild_count = self.inner.guilds.len();
        tracing::info!("running in {guild_count} guild(s)");
        self.bot.info().set_guild_count(guild_count);

        let mut set = JoinSet::new();

        self.inner.guilds.iter().for_each(|g| {
            let db = self.bot.db().clone();
            let g = g.id.get() as i64;
            set.spawn(async move {
                sqlx::query!(
                    r#"--sql
                    INSERT INTO guild_configs
                        (id)
                    SELECT $1
                    WHERE
                        NOT EXISTS (
                            SELECT 1 FROM guild_configs WHERE id = $1
                        );"#,
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
