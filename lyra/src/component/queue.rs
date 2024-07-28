mod clear;
mod fair_queue;
mod r#move;
mod play;
mod remove;
mod remove_range;
mod repeat;
mod shuffle;

pub use clear::Clear;
#[allow(clippy::module_name_repetitions)]
pub use fair_queue::FairQueue;
use lyra_ext::{
    num::usize_to_i64_truncating,
    pretty::{duration_display::DurationDisplay, join::PrettyJoiner, truncate::PrettyTruncator},
};

#[allow(clippy::module_name_repetitions)]
pub use play::AddToQueue;
pub use play::{Autocomplete as PlayAutocomplete, File as PlayFile, Play};
pub use r#move::{Autocomplete as MoveAutocomplete, Move};
pub use remove::{Autocomplete as RemoveAutocomplete, Remove};
pub use remove_range::{Autocomplete as RemoveRangeAutocomplete, RemoveRange};
pub use repeat::Repeat;
pub use shuffle::Shuffle;

use std::{collections::HashSet, num::NonZeroUsize, time::Duration};

use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;
use twilight_model::application::command::{CommandOptionChoice, CommandOptionChoiceValue};

use crate::{
    command::{
        macros::{note_fol, out},
        model::{GuildCtx, RespondViaMessage},
        require::PlayerInterface,
    },
    core::{
        model::{CacheAware, InteractionClient},
        r#const::{
            discord::COMMAND_CHOICES_LIMIT, misc::ADD_TRACKS_WRAP_LIMIT, text::FUZZY_MATCHER,
        },
    },
    error::{component::queue::RemoveTracksError, PositionOutOfRange as PositionOutOfRangeError},
    lavalink::{CorrectTrackInfo, QueueItem},
};

fn generate_position_choice(
    position: NonZeroUsize,
    track: &QueueItem,
    cx: &impl CacheAware,
) -> CommandOptionChoice {
    let track_info = &track.data().info;
    let track_length = Duration::from_millis(track_info.length);
    let requester = cx.cache().user(track.requester()).map_or_else(
        || String::from("Unknown User"),
        |u| {
            u.global_name
                .clone()
                .unwrap_or_else(|| u.name.clone())
                .pretty_truncate(12)
                .to_string()
        },
    );

    CommandOptionChoice {
        name: format!(
            "#{} ⌛{} 🎤{} 🎵{}",
            position,
            track_length.pretty_display(),
            requester,
            track_info.corrected_title().pretty_truncate(53)
        ),
        name_localizations: None,
        value: CommandOptionChoiceValue::Integer(usize_to_i64_truncating(position.get())),
    }
}

pub fn generate_position_choices<'a>(
    position: NonZeroUsize,
    queue_len: NonZeroUsize,
    queue_iter: impl Iterator<Item = (NonZeroUsize, &'a QueueItem)> + Clone,
    excluded: &HashSet<NonZeroUsize>,
    cx: &impl CacheAware,
) -> Vec<CommandOptionChoice> {
    impl_generate_position_choices(
        queue_iter
            .cycle()
            .skip_while(|(p, _)| *p < position)
            .take(queue_len.get()),
        excluded,
        cx,
    )
}

fn generate_position_choices_reversed<'a>(
    position: NonZeroUsize,
    queue_len: NonZeroUsize,
    queue_iter: impl Clone + DoubleEndedIterator<Item = (NonZeroUsize, &'a QueueItem)>,
    excluded: &HashSet<NonZeroUsize>,
    cx: &impl CacheAware,
) -> Vec<CommandOptionChoice> {
    impl_generate_position_choices(
        queue_iter
            .rev()
            .cycle()
            .skip_while(|(p, _)| *p > position)
            .take(queue_len.get()),
        excluded,
        cx,
    )
}

fn impl_generate_position_choices<'a>(
    queue_iter: impl Iterator<Item = (NonZeroUsize, &'a QueueItem)> + Clone,
    excluded: &HashSet<NonZeroUsize>,
    cx: &impl CacheAware,
) -> Vec<CommandOptionChoice> {
    queue_iter
        .filter(|(p, _)| !excluded.contains(p))
        .take(COMMAND_CHOICES_LIMIT)
        .map(|(p, t)| generate_position_choice(p, t, cx))
        .collect()
}

pub fn generate_position_choices_from_input<'a>(
    input: i64,
    queue_len: NonZeroUsize,
    queue_iter: impl Clone + DoubleEndedIterator<Item = (NonZeroUsize, &'a QueueItem)>,
    excluded: &HashSet<NonZeroUsize>,
    cx: &impl CacheAware,
) -> Vec<CommandOptionChoice> {
    normalize_queue_position(input, queue_len)
        .filter(|p| !excluded.contains(p))
        .map_or_else(Vec::new, |position| {
            if input.is_positive() {
                return generate_position_choices(position, queue_len, queue_iter, excluded, cx);
            }
            generate_position_choices_reversed(position, queue_len, queue_iter, excluded, cx)
        })
}

pub fn generate_position_choices_from_fuzzy_match<'a>(
    focused: &str,
    queue_iter: impl Iterator<Item = (NonZeroUsize, &'a QueueItem)>,
    excluded: &HashSet<NonZeroUsize>,
    cx: &impl CacheAware,
) -> Vec<CommandOptionChoice> {
    let queue_iter = queue_iter
        .filter_map(|(p, t)| {
            let track_info = &t.data().info;
            let author = track_info.corrected_author();
            let title = track_info.corrected_title();
            let requester = t.requester();
            Some((
                p,
                t,
                FUZZY_MATCHER.fuzzy_match(&format!("{requester} {author} {title}",), focused)?,
            ))
        })
        .sorted_by_key(|(_, _, s)| -s)
        .map(|(p, t, _)| (p, t));
    impl_generate_position_choices(queue_iter, excluded, cx)
}

fn normalize_queue_position(position: i64, queue_len: NonZeroUsize) -> Option<NonZeroUsize> {
    #[allow(clippy::cast_possible_truncation)]
    let position_usize = position.unsigned_abs() as usize;

    (1..=queue_len.get()).contains(&position_usize).then(|| {
        NonZeroUsize::new(
            position
                .is_positive()
                .then_some(position_usize)
                .unwrap_or_else(|| queue_len.get() - position_usize + 1),
        )
    })?
}

pub const fn validate_input_position(
    input: i64,
    queue_len: usize,
) -> Result<(), PositionOutOfRangeError> {
    if 1 > input || input > usize_to_i64_truncating(queue_len) {
        return Err(if queue_len == 1 {
            PositionOutOfRangeError::OnlyTrack(input)
        } else {
            PositionOutOfRangeError::OutOfRange {
                position: input,
                queue_len,
            }
        });
    }

    Ok(())
}

fn validate_input_positions(
    inputs: &[i64],
    queue_len: usize,
) -> Result<(), PositionOutOfRangeError> {
    inputs
        .iter()
        .try_for_each(|&input| validate_input_position(input, queue_len))?;

    Ok(())
}

async fn remove_range(
    start: i64,
    end: i64,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    player: &PlayerInterface,
) -> Result<(), RemoveTracksError> {
    #[allow(clippy::cast_possible_truncation)]
    let (start_usize, end_usize) = (start.unsigned_abs() as usize, end.unsigned_abs() as usize);

    let data = player.data();
    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();

    let range = (start_usize - 1)..end_usize;
    let queue_len = queue.len();
    let positions_len = (end_usize - start_usize) + 1;
    let queue_cleared = positions_len > 1 && positions_len == queue_len;
    let removed = if queue_cleared {
        queue.drain_all().collect::<Vec<_>>()
    } else {
        queue.drain(range).collect()
    };

    let positions = (start_usize..=end_usize)
        .filter_map(NonZeroUsize::new)
        .collect();

    drop(data_w);
    impl_remove(positions, removed, queue_cleared, ctx, player).await
}

async fn remove(
    positions: Box<[NonZeroUsize]>,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    player: &PlayerInterface,
) -> Result<(), RemoveTracksError> {
    let data = player.data();
    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();

    let queue_len = queue.len();
    let positions_len = positions.len();
    let queue_cleared = positions_len > 1 && positions_len == queue_len;
    let removed = if queue_cleared {
        queue.drain_all().collect::<Vec<_>>()
    } else {
        queue.dequeue(&positions).collect()
    };

    drop(data_w);
    impl_remove(positions, removed, queue_cleared, ctx, player).await
}

#[allow(clippy::significant_drop_tightening)]
async fn impl_remove(
    positions: Box<[NonZeroUsize]>,
    removed: Vec<QueueItem>,
    queue_cleared: bool,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    player: &PlayerInterface,
) -> Result<(), RemoveTracksError> {
    let data = player.data();
    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();

    let removed_len = removed.len();
    let removed_text = match removed_len {
        0 => String::new(),
        1..=ADD_TRACKS_WRAP_LIMIT => removed
            .into_iter()
            .map(|t| format!("`{}`", t.into_data().info.corrected_title()))
            .collect::<Box<[_]>>()
            .pretty_join_with_and(),
        _ => format!("`{removed_len} tracks`"),
    };
    let minus = match removed_len {
        // SAFETY: `/remove` always remove at least one track after input positions have been validated,
        //         so this branch is unreachable
        0 => unsafe { std::hint::unreachable_unchecked() },
        1 => "**`ー`**",
        _ => "**`≡-`**",
    };

    let current = queue.position();
    let before_current = positions.partition_point(|&i| i < current);
    *queue.index_mut() -= positions[..before_current].len();

    if positions.binary_search(&current).is_ok() {
        queue.downgrade_repeat_mode();
        let next = queue.current().map(QueueItem::data);

        if let Some(next) = next {
            queue.acquire_advance_lock();
            player.context.play_now(next).await?;
        } else {
            player.acquire_advance_lock_and_stop_with(queue).await?;
        }
    }

    out!(format!("{} Removed {}", minus, removed_text), ?ctx);

    if queue_cleared {
        let clear = InteractionClient::mention_command::<Clear>();

        note_fol!(
            format!("For clearing the entire queue, use {} instead.", clear),
            ctx
        );
    }
    Ok(())
}
