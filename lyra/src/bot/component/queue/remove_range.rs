use std::{
    collections::HashSet,
    num::{IntErrorKind, NonZeroUsize},
};

use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::bot::{
    command::{
        check,
        macros::{bad, sus},
        model::{BotAutocomplete, BotSlashCommand},
        AutocompleteCtx, SlashCtx,
    },
    component::queue::Remove,
    core::model::InteractionClient,
    error::command::{AutocompleteResult, Result as CommandResult},
    gateway::ExpectedGuildIdAware,
    lavalink::{DelegateMethods, LavalinkAware},
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
    ctx: &AutocompleteCtx,
) -> Vec<CommandOptionChoice> {
    let Some(data) = ctx.lavalink().get_player_data(ctx.guild_id()) else {
        return Vec::new();
    };
    let data_r = data.read().await;
    let (queue, Some(queue_len)) = (data_r.queue(), NonZeroUsize::new(data_r.queue().len())) else {
        return Vec::new();
    };

    let queue_iter = queue
        .iter()
        .enumerate()
        .filter_map(|(i, t)| NonZeroUsize::new(i + 1).map(|i| (i, t)));

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

    match options.focused.parse::<i64>() {
        Ok(input) => super::generate_position_choices_from_input(
            input, queue_len, queue_iter, &excluded, ctx,
        ),
        Err(e) if matches!(e.kind(), IntErrorKind::Empty) => match options.kind {
            RemoveRangeAutocompleteOptionsType::StartFocused
            | RemoveRangeAutocompleteOptionsType::StartFocusedEndCompleted(_) => {
                super::generate_position_choices(
                    queue.position(),
                    queue_len,
                    queue_iter,
                    &excluded,
                    ctx,
                )
            }
            RemoveRangeAutocompleteOptionsType::EndFocused
            | RemoveRangeAutocompleteOptionsType::EndFocusedStartCompleted(_) => {
                super::generate_position_choices_reversed(
                    queue_len, queue_len, queue_iter, &excluded, ctx,
                )
            }
        },
        Err(_) => super::generate_position_choices_from_fuzzy_match(
            &options.focused,
            queue_iter,
            &excluded,
            ctx,
        ),
    }
}

#[derive(CommandModel)]
#[command(autocomplete = true)]
pub struct Autocomplete {
    start: AutocompleteValue<i64>,
    end: AutocompleteValue<i64>,
}

impl BotAutocomplete for Autocomplete {
    async fn execute(self, mut ctx: AutocompleteCtx) -> AutocompleteResult {
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
            _ => unreachable!(),
        };

        let options = RemoveRangeAutocompleteOptions {
            focused: focused.into_boxed_str(),
            kind,
        };
        let choices = generate_remove_range_autocomplete_choices(&options, &ctx).await;
        Ok(ctx.autocomplete(choices).await?)
    }
}

/// Removes a range of tracks from the queue
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
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        let in_voice_with_user = check::in_voice(&ctx)?.with_user()?;
        check::queue_not_empty(&ctx).await?;
        check::not_suppressed(&ctx)?;

        let data = ctx.lavalink().player_data(ctx.guild_id());
        let data_r = data.read().await;
        let queue = data_r.queue();
        let queue_len = queue.len();

        if queue_len == 1 {
            let remove = InteractionClient::mention_command::<Remove>();

            drop(in_voice_with_user);
            sus!(
                format!("The queue only has one track; Use {remove} instead."),
                ctx
            );
        }

        super::validate_input_positions(&[self.start, self.end], queue_len)?;

        if self.end <= self.start {
            let message = if self.end == queue_len as i64 {
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

            drop(in_voice_with_user);
            bad!(message, ctx);
        }

        let positions = (self.start..=self.end).filter_map(|p| NonZeroUsize::new(p as usize));
        check::all_users_track(positions, in_voice_with_user, queue, &ctx)?;

        drop(data_r);
        Ok(super::remove_range(self.start, self.end, &mut ctx).await?)
    }
}
