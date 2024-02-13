use std::{
    collections::HashSet,
    num::{IntErrorKind, NonZeroUsize},
};

use itertools::Itertools;
use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::application::command::CommandOptionChoice;

use crate::bot::{
    command::{
        check,
        model::{AutocompleteCtx, BotAutocomplete, BotSlashCommand, SlashCommand},
        Ctx,
    },
    error::command::{AutocompleteResult, Result as CommandResult},
    gateway::ExpectedGuildIdAware,
    lavalink::ClientAware,
};

async fn generate_remove_choices(
    focused: &str,
    finished: Vec<i64>,
    ctx: &AutocompleteCtx,
) -> Vec<CommandOptionChoice> {
    let Some(data) = ctx.lavalink().get_player_data(ctx.guild_id()) else {
        return Vec::new();
    };
    let data_r = data.read().await;
    let (queue, Some(queue_len)) = (data_r.queue(), NonZeroUsize::new(data_r.queue().len())) else {
        return Vec::new();
    };

    let excluded = finished
        .into_iter()
        .filter_map(|i| super::normalize_queue_position(i, queue_len))
        .collect::<HashSet<_>>();

    let queue_iter = queue
        .iter()
        .enumerate()
        .filter_map(|(i, t)| NonZeroUsize::new(i + 1).map(|i| (i, t)));

    match focused.parse::<i64>() {
        Ok(input) => super::generate_position_choices_from_input(
            input, queue_len, queue_iter, &excluded, ctx,
        ),
        Err(e) if matches!(e.kind(), IntErrorKind::Empty) => super::generate_position_choices(
            queue.position(),
            queue_len,
            queue_iter,
            &excluded,
            ctx,
        ),
        Err(_) => {
            super::generate_position_choices_from_fuzzy_match(focused, queue_iter, &excluded, ctx)
        }
    }
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
    async fn execute(self, mut ctx: AutocompleteCtx) -> AutocompleteResult {
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
            .expect("at least one option must be focused");

        let choices = generate_remove_choices(&focused, finished, &ctx).await;
        Ok(ctx.autocomplete(choices).await?)
    }
}

/// Removes track(s) from the queue
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
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        let in_voice_with_user = check::in_voice(&ctx)?.with_user()?;
        check::queue_not_empty(&ctx).await?;
        check::not_suppressed(&ctx)?;

        let data = ctx.lavalink().player_data(ctx.guild_id());
        let data_r = data.read().await;
        let queue = data_r.queue();
        let queue_len = queue.len();

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

        super::validate_input_positions(&inputs, queue_len)?;

        let mut positions = inputs
            .iter()
            .filter_map(|p| NonZeroUsize::new(*p as usize))
            .collect::<Vec<_>>();

        check::all_users_track(positions.iter().copied(), in_voice_with_user, queue, &ctx)?;

        positions.sort_unstable();

        Ok(super::remove(positions.into(), &mut ctx).await?)
    }
}
