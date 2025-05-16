use std::{
    collections::HashSet,
    num::{IntErrorKind, NonZeroUsize},
};

use lyra_ext::num::usize_to_i64_truncating;
use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::{
    LavalinkAndGuildIdAware,
    command::{
        AutocompleteCtx, SlashCtx, check,
        model::{BotAutocomplete, BotSlashCommand},
        require,
    },
    component::queue::Remove,
    core::{
        http::InteractionClient,
        model::{
            CacheAware,
            response::initial::{
                autocomplete::RespondAutocomplete, message::create::RespondWithMessage,
            },
        },
    },
    error::{CommandResult, command::AutocompleteResult},
};

enum RemoveRangeAutocompleteOptionsType {
    StartFocused,
    EndFocused,
    StartFocusedEndCompleted(i64),
    EndFocusedStartCompleted(i64),
}

struct RemoveRangeAutocompleteOptions {
    focused: Box<str>,
    kind: RemoveRangeAutocompleteOptionsType,
}

async fn generate_remove_range_autocomplete_choices(
    options: &RemoveRangeAutocompleteOptions,
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

    let queue_iter = queue.iter_positions_and_items();

    let excluded = match options.kind {
        RemoveRangeAutocompleteOptionsType::StartFocused
        | RemoveRangeAutocompleteOptionsType::EndFocused => HashSet::new(),
        RemoveRangeAutocompleteOptionsType::StartFocusedEndCompleted(end) => {
            let Some(end) = super::normalize_queue_position(end, queue_len) else {
                return Vec::new();
            };

            (end.get()..=queue_len.get())
                .filter_map(NonZeroUsize::new)
                .collect()
        }
        RemoveRangeAutocompleteOptionsType::EndFocusedStartCompleted(start) => {
            let Some(start) = super::normalize_queue_position(start, queue_len) else {
                return Vec::new();
            };

            (1..=start.get()).filter_map(NonZeroUsize::new).collect()
        }
    };

    let choices = match options.focused.parse::<i64>() {
        Ok(input) => {
            super::generate_position_choices_from_input(input, queue_len, queue_iter, &excluded, cx)
        }
        Err(e) if matches!(e.kind(), IntErrorKind::Empty) => match options.kind {
            RemoveRangeAutocompleteOptionsType::StartFocused
            | RemoveRangeAutocompleteOptionsType::StartFocusedEndCompleted(_) => {
                super::generate_position_choices(
                    queue.position(),
                    queue_len,
                    queue_iter,
                    &excluded,
                    cx,
                )
            }
            RemoveRangeAutocompleteOptionsType::EndFocused
            | RemoveRangeAutocompleteOptionsType::EndFocusedStartCompleted(_) => {
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
    };
    drop(data_r);
    choices
}

#[derive(CommandModel)]
#[command(autocomplete = true)]
pub struct Autocomplete {
    start: AutocompleteValue<i64>,
    end: AutocompleteValue<i64>,
}

impl BotAutocomplete for Autocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult {
        let mut ctx = require::guild(ctx)?;
        let (focused, kind) = match (self.start, self.end) {
            (AutocompleteValue::Focused(focused), AutocompleteValue::None) => {
                (focused, RemoveRangeAutocompleteOptionsType::StartFocused)
            }
            (AutocompleteValue::None, AutocompleteValue::Focused(focused)) => {
                (focused, RemoveRangeAutocompleteOptionsType::EndFocused)
            }
            (AutocompleteValue::Focused(focused), AutocompleteValue::Completed(i)) => (
                focused,
                RemoveRangeAutocompleteOptionsType::StartFocusedEndCompleted(i),
            ),
            (AutocompleteValue::Completed(i), AutocompleteValue::Focused(focused)) => (
                focused,
                RemoveRangeAutocompleteOptionsType::EndFocusedStartCompleted(i),
            ),
            _ => panic!("not exactly one autocomplete option focused"),
        };

        let options = RemoveRangeAutocompleteOptions {
            focused: focused.into_boxed_str(),
            kind,
        };
        let choices = generate_remove_range_autocomplete_choices(&options, &ctx).await;
        ctx.autocomplete(choices).await?;
        Ok(())
    }
}

/// Removes a range of tracks from the queue.
#[derive(CommandModel, CreateCommand)]
#[command(name = "remove-range", dm_permission = false)]
pub struct RemoveRange {
    /// Which starting tracks? [track title / position in queue]
    #[command(min_value = 1, autocomplete = true)]
    start: i64,
    /// Which starting tracks? [track title / position in queue]
    #[command(min_value = 1, autocomplete = true)]
    end: i64,
}

impl BotSlashCommand for RemoveRange {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;

        let data = player.data();
        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;

        let queue_len = queue.len();
        if queue_len == 1 {
            let remove = InteractionClient::mention_command::<Remove>();

            ctx.susp(format!(
                "The queue only has one track; Use {remove} instead."
            ))
            .await?;
            return Ok(());
        }

        super::validate_input_position(self.start, queue_len)?;
        super::validate_input_position(self.end, queue_len)?;

        if self.end <= self.start {
            let message = if self.end == usize_to_i64_truncating(queue_len) {
                format!(
                    "Invalid starting position: `{}`; Starting position must be from `1` to `{}`.",
                    self.start,
                    queue_len - 1
                )
            } else {
                format!(
                    "Invalid ending position: `{}`; Ending position must be from `{}` to `{}`.",
                    self.end,
                    self.start + 1,
                    queue_len
                )
            };

            ctx.wrng(message).await?;
            return Ok(());
        }

        #[allow(clippy::cast_possible_truncation)]
        let positions =
            (self.start..=self.end).filter_map(|p| NonZeroUsize::new(p.unsigned_abs() as usize));
        check::all_users_track(queue, positions, in_voice_with_user)?;

        drop(data_r);
        Ok(super::remove_range(self.start, self.end, &mut ctx, &player).await?)
    }
}
