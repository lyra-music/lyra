use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        check,
        macros::{bad, hid, out},
        model::{BotSlashCommand, SlashCommand},
        Ctx,
    },
    error::command::Result as CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{ClientAware, QueueIndexerType},
};

/// Toggles fair queuing
#[derive(CommandModel, CreateCommand)]
#[command(name = "fair-queue", dm_permission = false)]
pub struct FairQueue;

impl BotSlashCommand for FairQueue {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        check::user_is_dj(&ctx)?;
        check::in_voice(&ctx)?.with_someone_else()?;
        check::queue_not_empty(&ctx).await?;

        let guild_id = ctx.guild_id();
        let data = ctx.lavalink().player_data(guild_id);
        let indexer_type = data.read().await.queue().indexer_type();

        match indexer_type {
            QueueIndexerType::Fair => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(QueueIndexerType::Standard);
                out!("**` ⮆ `** Disabled fair queue", ctx);
            }
            QueueIndexerType::Shuffled => {
                bad!(
                    "Cannot enable fair queue as shuffle is currently enabled",
                    ctx
                );
            }
            QueueIndexerType::Standard => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(QueueIndexerType::Fair);
                out!("⚖️ Enabled fair queue", ctx);
            }
        }
    }
}
