use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{SlashCtx, check, model::BotSlashCommand, require},
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
    lavalink::IndexerType,
};

/// Toggles fair queuing.
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
                    .set_indexer_then_update_and_apply_to_now_playing(IndexerType::Standard)
                    .await?;

                ctx.out("**` ⮆ `** Disabled fair queue.").await?;
                Ok(())
            }
            IndexerType::Shuffled => {
                ctx.wrng("Cannot enable fair queue as shuffle is currently enabled.")
                    .await?;
                Ok(())
            }
            IndexerType::Standard => {
                data.write()
                    .await
                    .set_indexer_then_update_and_apply_to_now_playing(IndexerType::Fair)
                    .await?;

                ctx.out("⚖️ Enabled fair queue.").await?;
                Ok(())
            }
        }
    }
}
