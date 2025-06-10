use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        SlashCtx, check,
        model::{BotSlashCommand, GuildCtx, RespondViaMessage},
        require,
        util::controller_fmt,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::{CommandResult, component::queue::shuffle::ShuffleError},
    lavalink::{IndexerType, OwnedPlayerData},
};

/// Toggles queue shuffling.
#[derive(CommandModel, CreateCommand)]
#[command(name = "shuffle", contexts = "guild")]
pub struct Shuffle;

impl BotSlashCommand for Shuffle {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let player = require::player(&ctx)?;
        let data = player.data();
        require::queue_not_empty(&data.read().await)?;

        Ok(shuffle(data, &mut ctx, false).await?)
    }
}

pub async fn shuffle(
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), ShuffleError> {
    // FAIRNESS: if a member requests to enable or disable shuffle, they need to be
    // the only person in voice, as modifying the queue indexing order will be
    // unfair to everyone who queued after this current track: the tracks after the
    // current track will be delayed for an unspecified amount of time.
    check::user_in(require::in_voice(ctx)?)?.only()?;

    let indexer_type = data.read().await.queue().indexer_type();
    match indexer_type {
        IndexerType::Shuffled => {
            data.write()
                .await
                .set_indexer_then_update_and_apply_to_now_playing(IndexerType::Standard)
                .await?;

            let content = controller_fmt(ctx, via_controller, "**` ⮆ `** Disabled shuffle.");
            ctx.out(content).await?;
            Ok(())
        }
        IndexerType::Fair => {
            ctx.wrng(
                // The shuffle button on the playback controller will be disabled, so no need to use `controller_fmt` here
                "Cannot enable shuffle as fair queue is currently enabled.",
            )
            .await?;
            Ok(())
        }
        IndexerType::Standard => {
            data.write()
                .await
                .set_indexer_then_update_and_apply_to_now_playing(IndexerType::Shuffled)
                .await?;

            let content = controller_fmt(ctx, via_controller, "🔀 Enabled shuffle.");
            ctx.out(content).await?;
            Ok(())
        }
    }
}
