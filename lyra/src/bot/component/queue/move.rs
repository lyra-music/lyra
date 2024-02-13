use std::{collections::HashSet, num::NonZeroUsize};

use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::bot::{
    command::{
        check,
        macros::{bad, hid, out, sus},
        model::{AutocompleteCtx, BotAutocomplete, BotSlashCommand, SlashCommand},
        Ctx,
    },
    component::queue::normalize_queue_position,
    error::command::{AutocompleteResult, Result as CommandResult},
    gateway::ExpectedGuildIdAware,
    lavalink::ClientAware,
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

async fn generate_move_autocomplete_choices(
    options: &MoveAutocompleteOptions,
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
        Ok(input) => super::generate_position_choices_from_input(
            input, queue_len, queue_iter, &excluded, ctx,
        ),
        Err(e) if matches!(e.kind(), std::num::IntErrorKind::Empty) => match options.kind {
            MoveAutocompleteOptionType::TrackFocused
            | MoveAutocompleteOptionType::TrackFocusedPositionCompleted(_) => {
                super::generate_position_choices(
                    queue.position(),
                    queue_len,
                    queue_iter,
                    &excluded,
                    ctx,
                )
            }
            MoveAutocompleteOptionType::PositionFocused
            | MoveAutocompleteOptionType::PositionFocusedTrackCompleted(_) => {
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
    track: AutocompleteValue<i64>,
    position: AutocompleteValue<i64>,
}

impl BotAutocomplete for Autocomplete {
    async fn execute(self, mut ctx: AutocompleteCtx) -> AutocompleteResult {
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
            _ => unreachable!(),
        };

        let options = MoveAutocompleteOptions {
            focused: focused.into_boxed_str(),
            kind,
        };
        let choices = generate_move_autocomplete_choices(&options, &ctx).await;
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
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        let in_voice_with_user = check::in_voice(&ctx)?.with_user()?;
        check::queue_not_empty(&ctx).await?;
        check::not_suppressed(&ctx)?;

        let data = ctx.lavalink().player_data(ctx.guild_id());
        let mut data_w = data.write().await;
        let queue = data_w.queue_mut();
        let queue_len = queue.len();

        if queue_len == 1 {
            drop(in_voice_with_user);
            sus!(
                "Nowhere to move the track as it is the only track in the queue.",
                ctx
            );
        }

        super::validate_input_positions(&[self.track, self.position], queue_len)?;

        if self.track == self.position {
            drop(in_voice_with_user);
            bad!(
                format!("Invalid new position: {}; New position must be different from the old position", self.position),
                ctx
            );
        }

        let position = NonZeroUsize::new(self.position as usize)
            .expect("`self.position as usize` must be nonzero");
        check::users_track(position, in_voice_with_user, queue, &ctx)?;

        let track_position =
            NonZeroUsize::new(self.track as usize).expect("`self.track as usize` must be nonzero");
        let queue_position = queue.position();

        let track = queue
            .remove(track_position.get() - 1)
            .expect("`self.track as usize - 1` must be in bounds");
        let track_title = track.track().info.title.clone();

        let insert_position = position.get() - 1;

        *queue.index_mut() = if track_position == queue_position {
            insert_position
        } else if track_position < position {
            queue.index().saturating_sub(1)
        } else {
            queue.index().saturating_add(1)
        };

        queue.insert(insert_position, track);

        out!(
            format!("⤴️ Moved `{}` to position **`{position}`**", track_title),
            ctx
        );
    }
}
