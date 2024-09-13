use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        macros::{bad, out},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    error::CommandResult,
    lavalink::IndexerType,
};

/// Toggles fair queuing
#[derive(CommandModel, CreateCommand)]
#[command(name = "fair-queue", dm_permission = false)]
pub struct FairQueue;

impl BotSlashCommand for FairQueue {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        check::user_is_dj(&ctx)?;
        let _ = require::in_voice(&ctx)?.and_with_someone_else()?;
        let data = require::player(&ctx)?.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let indexer_type = queue.indexer_type();
        drop(data_r);

        match indexer_type {
            IndexerType::Fair => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(IndexerType::Standard);
                out!("**` ⮆ `** Disabled fair queue", ctx);
            }
            IndexerType::Shuffled => {
                bad!(
                    "Cannot enable fair queue as shuffle is currently enabled",
                    ctx
                );
            }
            IndexerType::Standard => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(IndexerType::Fair);
                out!("⚖️ Enabled fair queue", ctx);
            }
        }
    }
}
