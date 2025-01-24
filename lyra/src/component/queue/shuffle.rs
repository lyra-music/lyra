use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        macros::{bad, out},
        model::{BotSlashCommand, GuildCtx, RespondViaMessage},
        require,
        util::controller_fmt,
        SlashCtx,
    },
    error::CommandResult,
    lavalink::{IndexerType, OwnedPlayerData},
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
        require::queue_not_empty(&data_r)?;
        drop(data_r);

        Ok(shuffle(data, &mut ctx, false).await?)
    }
}

pub async fn shuffle(
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), crate::error::component::queue::ShuffleError> {
    let indexer_type = data.read().await.queue().indexer_type();
    match indexer_type {
        IndexerType::Shuffled => {
            data.write()
                .await
                .set_indexer_then_update_and_apply_to_now_playing(IndexerType::Standard)
                .await?;

            let content = controller_fmt(ctx, via_controller, "**` ⮆ `** Disabled shuffle");
            out!(content, ?ctx);
            Ok(())
        }
        IndexerType::Fair => {
            bad!(
                // The shuffle button on the playback controller will be disabled, so no need to use `controller_fmt` here
                "Cannot enable shuffle as fair queue is currently enabled",
                ctx
            );
        }
        IndexerType::Standard => {
            data.write()
                .await
                .set_indexer_then_update_and_apply_to_now_playing(IndexerType::Shuffled)
                .await?;

            let content = controller_fmt(ctx, via_controller, "🔀 Enabled shuffle");
            out!(content, ?ctx);
            Ok(())
        }
    }
}
