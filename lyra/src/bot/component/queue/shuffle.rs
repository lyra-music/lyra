use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        check::CheckerBuilder,
        macros::{bad, hid, out},
        model::{BotSlashCommand, SlashCommand},
        Ctx,
    },
    error::command::Result as CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{ClientAware, QueueIndexerType},
};

/// Toggles queue shuffling
#[derive(CommandModel, CreateCommand)]
#[command(name = "shuffle", dm_permission = false)]
pub struct Shuffle;

impl BotSlashCommand for Shuffle {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        CheckerBuilder::new()
            .in_voice_with_user_only()
            .queue_not_empty()
            .build()
            .run(&mut ctx)
            .await?;

        let guild_id = ctx.guild_id();
        let data = ctx.lavalink().player_data(guild_id);
        let data_r = data.read().await;
        let indexer_type = data_r.queue().indexer_type();

        match indexer_type {
            QueueIndexerType::Shuffled => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(QueueIndexerType::Standard);
                out!("**` ⮆ `** Disabled shuffle", ctx);
            }
            QueueIndexerType::Fair => {
                bad!(
                    "Cannot enable shuffle as fair queue is currently enabled",
                    ctx
                );
            }
            QueueIndexerType::Standard => {
                data.write()
                    .await
                    .queue_mut()
                    .set_indexer_type(QueueIndexerType::Shuffled);
                out!("🔀 Enabled shuffle", ctx);
            }
        }
    }
}
