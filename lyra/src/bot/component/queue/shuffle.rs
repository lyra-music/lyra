use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
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
        check::in_voice_with_user(require::in_voice(&ctx)?)?.only()?;
        let player = require::player(&ctx)?.and_queue_not_empty().await?;

        let data = player.data();
        let data_r = data.read().await;
        let indexer_type = data_r.queue().indexer_type();

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
