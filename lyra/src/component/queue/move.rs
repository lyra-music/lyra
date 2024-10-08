use std::{collections::HashSet, num::NonZeroUsize};

use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::{
    command::{
        check,
        macros::{bad, out, sus},
        model::{BotAutocomplete, BotSlashCommand},
        require, AutocompleteCtx, SlashCtx,
    },
    component::queue::normalize_queue_position,
    core::model::CacheAware,
    error::{command::AutocompleteResult, CommandResult},
    lavalink::CorrectTrackInfo,
    LavalinkAndGuildIdAware,
};

enum MoveAutocompleteOptionType {
    TrackFocused,
    PositionFocused,
    TrackFocusedPositionCompleted(i64),
    PositionFocusedTrackCompleted(i64),
}

struct MoveAutocompleteOptions {
    focused: Box<str>,
    kind: MoveAutocompleteOptionType,
}

#[allow(clippy::significant_drop_tightening)]
async fn generate_move_choices(
    options: &MoveAutocompleteOptions,
    cx: &(impl LavalinkAndGuildIdAware + CacheAware + Sync),
) -> Vec<CommandOptionChoice> {
    let Ok(player) = require::player(cx) else {
        return Vec::new();
    };
    let data = player.data();
    let data_r = data.read().await;
    let (queue, Some(queue_len)) = (data_r.queue(), NonZeroUsize::new(data_r.queue().len())) else {
        return Vec::new();
    };

    let queue_iter = queue.iter_positions_and_items();

    let excluded = match options.kind {
        MoveAutocompleteOptionType::TrackFocused | MoveAutocompleteOptionType::PositionFocused => {
            HashSet::new()
        }
        MoveAutocompleteOptionType::TrackFocusedPositionCompleted(position) => {
            let Some(position) = normalize_queue_position(position, queue_len) else {
                return Vec::new();
            };

            HashSet::from([position])
        }
        MoveAutocompleteOptionType::PositionFocusedTrackCompleted(track) => {
            let Some(track) = normalize_queue_position(track, queue_len) else {
                return Vec::new();
            };

            HashSet::from([track])
        }
    };

    match options.focused.parse::<i64>() {
        Ok(input) => {
            super::generate_position_choices_from_input(input, queue_len, queue_iter, &excluded, cx)
        }
        Err(e) if matches!(e.kind(), std::num::IntErrorKind::Empty) => match options.kind {
            MoveAutocompleteOptionType::TrackFocused
            | MoveAutocompleteOptionType::TrackFocusedPositionCompleted(_) => {
                super::generate_position_choices(
                    queue.position(),
                    queue_len,
                    queue_iter,
                    &excluded,
                    cx,
                )
            }
            MoveAutocompleteOptionType::PositionFocused
            | MoveAutocompleteOptionType::PositionFocusedTrackCompleted(_) => {
                super::generate_position_choices_reversed(
                    queue_len, queue_len, queue_iter, &excluded, cx,
                )
            }
        },
        Err(_) => super::generate_position_choices_from_fuzzy_match(
            &options.focused,
            queue_iter,
            &excluded,
            cx,
        ),
    }
}

#[derive(CommandModel)]
#[command(autocomplete = true)]
pub struct Autocomplete {
    track: AutocompleteValue<i64>,
    position: AutocompleteValue<i64>,
}

impl BotAutocomplete for Autocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult {
        let mut ctx = require::guild(ctx)?;
        let (focused, kind) = match (self.track, self.position) {
            (AutocompleteValue::Focused(focused), AutocompleteValue::None) => {
                (focused, MoveAutocompleteOptionType::TrackFocused)
            }
            (AutocompleteValue::None, AutocompleteValue::Focused(focused)) => {
                (focused, MoveAutocompleteOptionType::PositionFocused)
            }
            (AutocompleteValue::Focused(focused), AutocompleteValue::Completed(i)) => (
                focused,
                MoveAutocompleteOptionType::TrackFocusedPositionCompleted(i),
            ),
            (AutocompleteValue::Completed(i), AutocompleteValue::Focused(focused)) => (
                focused,
                MoveAutocompleteOptionType::PositionFocusedTrackCompleted(i),
            ),
            // SAFETY: only one autocomplete options can be focused at a time,
            //         so this branch is unreachable
            _ => unsafe { std::hint::unreachable_unchecked() },
        };

        let options = MoveAutocompleteOptions {
            focused: focused.into_boxed_str(),
            kind,
        };
        let choices = generate_move_choices(&options, &ctx).await;
        Ok(ctx.autocomplete(choices).await?)
    }
}

/// Moves a track to a new position in the queue
#[derive(CreateCommand, CommandModel)]
#[command(name = "move", dm_permission = false)]
pub struct Move {
    /// Move which track? [track title / position in queue]
    #[command(min_value = 1, autocomplete = true)]
    track: i64,
    /// ... to where? [track title / position in queue]
    #[command(min_value = 1, autocomplete = true)]
    position: i64,
}

impl BotSlashCommand for Move {
    #[allow(clippy::significant_drop_tightening)]
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;

        let data = player.data();
        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        let queue_len = queue.len();

        if queue_len == 1 {
            sus!(
                "Nowhere to move the track as it is the only track in the queue.",
                ctx
            );
        }

        super::validate_input_position(self.track, queue_len)?;
        super::validate_input_position(self.position, queue_len)?;

        if self.track == self.position {
            bad!(
                format!("Invalid new position: {}; New position must be different from the old position", self.position),
                ctx
            );
        }

        #[allow(clippy::cast_possible_truncation)]
        let (track_usize, position_usize) = (
            self.track.unsigned_abs() as usize,
            self.position.unsigned_abs() as usize,
        );

        // SAFETY: `self.track as usize` is in range [1, +inf), so it is non-zero
        let position = unsafe { NonZeroUsize::new_unchecked(position_usize) };
        let track = &queue[position];
        check::track_is_users(track, position, in_voice_with_user)?;

        // SAFETY: `self.track as usize` is in range [1, +inf), so it is non-zero
        let track_position = unsafe { NonZeroUsize::new_unchecked(track_usize) };
        let queue_position = queue.position();

        // SAFETY: `track_position.get() - 1` has been validated to be in-bounds, so this unwrap is safe
        let track = unsafe { queue.remove(track_position.get() - 1).unwrap_unchecked() };
        let track_title = track.data().info.corrected_title();
        let message = format!("⤴️ Moved `{track_title}` to position **`{position}`**");

        let insert_position = position.get() - 1;

        *queue.index_mut() = if track_position == queue_position {
            insert_position
        } else if track_position < position {
            queue.index().saturating_sub(1)
        } else {
            queue.index().saturating_add(1)
        };

        queue.insert(insert_position, track);

        out!(message, ctx);
    }
}
