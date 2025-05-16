use std::{
    collections::HashSet,
    num::{IntErrorKind, NonZeroUsize},
};

use itertools::Itertools;
use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::{
    LavalinkAndGuildIdAware,
    command::{
        AutocompleteCtx, SlashCtx, check,
        model::{BotAutocomplete, BotSlashCommand},
        require,
    },
    core::model::{CacheAware, response::initial::autocomplete::RespondAutocomplete},
    error::{CommandResult, command::AutocompleteResult},
};

async fn generate_remove_choices(
    focused: &str,
    finished: Vec<i64>,
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

    let excluded = finished
        .into_iter()
        .filter_map(|i| super::normalize_queue_position(i, queue_len))
        .collect::<HashSet<_>>();

    let queue_iter = queue.iter_positions_and_items();

    let choices = match focused.parse::<i64>() {
        Ok(input) => {
            super::generate_position_choices_from_input(input, queue_len, queue_iter, &excluded, cx)
        }
        Err(e) if matches!(e.kind(), IntErrorKind::Empty) => {
            super::generate_position_choices(queue.position(), queue_len, queue_iter, &excluded, cx)
        }
        Err(_) => {
            super::generate_position_choices_from_fuzzy_match(focused, queue_iter, &excluded, cx)
        }
    };
    drop(data_r);
    choices
}

#[derive(CommandModel)]
#[command(autocomplete = true)]
pub struct Autocomplete {
    track: AutocompleteValue<i64>,
    track_2: AutocompleteValue<i64>,
    track_3: AutocompleteValue<i64>,
    track_4: AutocompleteValue<i64>,
    track_5: AutocompleteValue<i64>,
}

impl BotAutocomplete for Autocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult {
        let mut ctx = require::guild(ctx)?;
        let tracks = [
            self.track,
            self.track_2,
            self.track_3,
            self.track_4,
            self.track_5,
        ];

        let finished = tracks
            .iter()
            .filter_map(|a| match a {
                AutocompleteValue::Completed(i) => Some(i),
                _ => None,
            })
            .copied()
            .collect::<Vec<_>>();
        let focused = tracks
            .into_iter()
            .find_map(|a| match a {
                AutocompleteValue::Focused(i) => Some(i),
                _ => None,
            })
            .expect("exactly one autocomplete option should be focused");

        let choices = generate_remove_choices(&focused, finished, &ctx).await;
        ctx.autocomplete(choices).await?;
        Ok(())
    }
}

/// Removes track(s) from the queue.
#[derive(CommandModel, CreateCommand)]
#[command(name = "remove", dm_permission = false)]
pub struct Remove {
    /// Which track? [track title / position in queue]
    #[command(min_value = 1, autocomplete = true)]
    track: i64,
    /// Which track? [track title / position in queue] (2)
    #[command(min_value = 1, autocomplete = true)]
    track_2: Option<i64>,
    /// Which track? [track title / position in queue] (3)
    #[command(min_value = 1, autocomplete = true)]
    track_3: Option<i64>,
    /// Which track? [track title / position in queue] (4)
    #[command(min_value = 1, autocomplete = true)]
    track_4: Option<i64>,
    /// Which track? [track title / position in queue] (5)
    #[command(min_value = 1, autocomplete = true)]
    track_5: Option<i64>,
}

impl BotSlashCommand for Remove {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;

        let player = require::player(&ctx)?;
        let data = player.data();
        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;

        let inputs = [
            Some(self.track),
            self.track_2,
            self.track_3,
            self.track_4,
            self.track_5,
        ]
        .into_iter()
        .flatten()
        .unique()
        .collect::<Box<_>>();

        super::validate_input_positions(&inputs, queue.len())?;

        #[allow(clippy::cast_possible_truncation)]
        let mut positions = inputs
            .iter()
            .filter_map(|&p| NonZeroUsize::new(p.unsigned_abs() as usize))
            .collect::<Vec<_>>();

        check::all_users_track(queue, positions.iter().copied(), in_voice_with_user)?;

        positions.sort_unstable();

        drop(data_r);
        Ok(super::remove(positions.into(), &mut ctx, &player).await?)
    }
}
