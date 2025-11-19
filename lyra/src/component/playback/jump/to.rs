use std::{
    collections::HashSet,
    num::{IntErrorKind, NonZeroUsize},
};

use lyra_ext::num::cast::i64_as_usize;
use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::{
    LavalinkAndGuildIdAware,
    command::{
        check,
        model::{
            BotGuildAutocomplete, BotGuildSlashCommand, GuildAutocompleteCtx, GuildSlashCmdCtx,
        },
        require,
    },
    component::queue::{
        generate_position_choices, generate_position_choices_from_fuzzy_match,
        generate_position_choices_from_input, validate_input_position,
    },
    core::model::{
        CacheAware,
        response::initial::{
            autocomplete::RespondAutocomplete, message::create::RespondWithMessage,
        },
    },
};

async fn generate_skip_to_choices(
    track: String,
    cx: &(impl CacheAware + LavalinkAndGuildIdAware + Sync),
) -> Vec<CommandOptionChoice> {
    let Ok(player) = require::player(cx) else {
        return Vec::new();
    };
    let data = player.data();
    let data_r = data.read().await;
    let (queue, Some(queue_len)) = (data_r.queue(), NonZeroUsize::new(data_r.queue().len())) else {
        return Vec::new();
    };

    let excluded = HashSet::from([queue.position()]);
    let queue_iter = queue.iter_positions_and_items();

    let choices = match track.parse::<i64>() {
        Ok(input) => {
            generate_position_choices_from_input(input, queue_len, queue_iter, &excluded, cx)
        }
        Err(e) if matches!(e.kind(), IntErrorKind::Empty) => {
            generate_position_choices(queue.position(), queue_len, queue_iter, &excluded, cx)
        }
        Err(_) => generate_position_choices_from_fuzzy_match(&track, queue_iter, &excluded, cx),
    };
    drop(data_r);
    choices
}

#[derive(CommandModel)]
#[command(autocomplete = true)]
pub struct Autocomplete {
    track: AutocompleteValue<i64>,
}

impl BotGuildAutocomplete for Autocomplete {
    async fn execute(
        self,
        mut ctx: GuildAutocompleteCtx,
    ) -> crate::error::command::AutocompleteResult {
        let AutocompleteValue::Focused(track) = self.track else {
            panic!("not exactly one autocomplete option focused")
        };

        let choices = generate_skip_to_choices(track, &ctx).await;
        ctx.autocomplete(choices).await?;
        Ok(())
    }
}

/// Jumps to a new track in the queue, skipping all track in-between.
#[derive(CommandModel, CreateCommand)]
#[command(name = "to")]
pub struct To {
    /// Which track? [track title / position in queue]
    #[command(min_value = 1, autocomplete = true)]
    track: i64,
}

impl BotGuildSlashCommand for To {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        let current_track = require::current_track(queue);
        if let Ok(ref curr) = current_track {
            check::current_track_is_users(curr, in_voice_with_user)?;
        }
        let current_track_exist = current_track.is_ok();

        let queue_len = queue.len();
        if queue_len == 1 {
            ctx.wrng("No where else to jump to.").await?;
            return Ok(());
        }

        let input = self.track;
        validate_input_position(input, queue_len)?;

        let position = i64_as_usize(input);
        if position == queue.position().get() {
            ctx.wrng("Cannot jump to the current track.").await?;
            return Ok(());
        }

        queue.downgrade_repeat_mode();
        if current_track_exist {
            // CORRECTNESS: the current track is present and will be ending via the
            // `cleanup_now_playing_message_and_play` call later, so this is correct
            queue.disable_advancing();
        }

        let index = position - 1;
        let mapped_index = queue.map_index_expected(index);
        let track = queue[mapped_index].data();
        ctx.out(format!(
            "↔️ Jumped to `{}` (`#{}`).",
            track.info.title, mapped_index
        ))
        .await?;
        *queue.index_mut() = index;
        player
            .cleanup_now_playing_message_and_play(&ctx, mapped_index, &mut data_w)
            .await?;

        drop(data_w);
        Ok(())
    }
}
