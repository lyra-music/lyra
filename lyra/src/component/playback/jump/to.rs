use std::{
    collections::HashSet,
    num::{IntErrorKind, NonZeroUsize},
};

use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::{
    LavalinkAndGuildIdAware,
    command::{
        check,
        macros::{bad, out},
        model::{BotAutocomplete, BotSlashCommand},
        require,
    },
    component::queue::{
        generate_position_choices, generate_position_choices_from_fuzzy_match,
        generate_position_choices_from_input, validate_input_position,
    },
    core::model::CacheAware,
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

impl BotAutocomplete for Autocomplete {
    async fn execute(
        self,
        ctx: crate::command::AutocompleteCtx,
    ) -> crate::error::command::AutocompleteResult {
        let mut ctx = require::guild(ctx)?;
        let AutocompleteValue::Focused(track) = self.track else {
            panic!("not exactly one autocomplete option focused")
        };

        let choices = generate_skip_to_choices(track, &ctx).await;
        Ok(ctx.autocomplete(choices).await?)
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

impl BotSlashCommand for To {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        if let Ok(current_track) = require::current_track(queue) {
            check::current_track_is_users(&current_track, in_voice_with_user)?;
        }

        let queue_len = queue.len();
        if queue_len == 1 {
            bad!("No where else to jump to.", ctx);
        }

        let input = self.track;
        validate_input_position(input, queue_len)?;

        #[allow(clippy::cast_possible_truncation)]
        let position = input.unsigned_abs() as usize;
        if position == queue.position().get() {
            bad!("Cannot jump to the current track.", ctx);
        }

        queue.downgrade_repeat_mode();
        queue.acquire_advance_lock();

        let index = position - 1;
        let track = queue[index].data();
        let txt = format!("↔️ Jumped to `{}` (`#{}`).", track.info.title, position);
        player.context.play_now(track).await?;

        *queue.index_mut() = index;
        drop(data_w);
        out!(txt, ctx);
    }
}
