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

/// Toggles queue shuffling
#[derive(CommandModel, CreateCommand)]
#[command(name = "shuffle", dm_permission = false)]
pub struct Shuffle;

impl BotSlashCommand for Shuffle {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        check::user_in(require::in_voice(&ctx)?)?.only()?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let indexer_type = queue.indexer_type();
        drop(data_r);

        match indexer_type {
            IndexerType::Shuffled => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(IndexerType::Standard);
                out!("**` ⮆ `** Disabled shuffle", ctx);
            }
            IndexerType::Fair => {
                bad!(
                    "Cannot enable shuffle as fair queue is currently enabled",
                    ctx
                );
            }
            IndexerType::Standard => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(IndexerType::Shuffled);
                out!("🔀 Enabled shuffle", ctx);
            }
        }
    }
}
