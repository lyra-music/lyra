mod down;
mod set;
mod up;

use std::num::NonZeroI64;

use lavalink_rs::{
    error::LavalinkResult,
    model::player::{Filters, Timescale},
};
use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    gateway::ExpectedGuildIdAware,
    lavalink::{DelegateMethods, LavalinkAware, Pitch as PitchModel},
};

enum Tier {
    Default,
    High,
    Low,
}

impl Tier {
    const fn emoji(&self) -> &'static str {
        match self {
            Self::Default => "🧑",
            Self::Low => "🐋",
            Self::High => "🦇",
        }
    }
}

impl PitchModel {
    fn tier(&self) -> Tier {
        match self.checked_get() {
            None => Tier::Default,
            Some(0.0..=1.0) => Tier::Low,
            _ => Tier::High,
        }
    }
}

async fn shift_pitch(
    ctx: &(impl LavalinkAware + ExpectedGuildIdAware + Sync),
    half_tones: NonZeroI64,
) -> LavalinkResult<(PitchModel, PitchModel)> {
    let guild_id = ctx.guild_id();
    let lavalink = ctx.lavalink();
    let player = lavalink.player(guild_id);
    let old_filter = player.get_player().await?.filters.unwrap_or_default();

    let (old_pitch, new_pitch) = lavalink
        .player_data(guild_id)
        .write()
        .await
        .pitch_mut()
        .clone_before_and_after_shifted(half_tones);
    let pitch = new_pitch.checked_get();

    let timescale = Some(Timescale {
        pitch,
        ..old_filter.timescale.unwrap_or_default()
    });

    player
        .set_filters(Filters {
            timescale,
            ..old_filter
        })
        .await?;
    Ok((old_pitch, new_pitch))
}

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "pitch", desc = ".")]
pub enum Pitch {
    #[command(name = "up")]
    Up(up::Up),
    #[command(name = "down")]
    Down(down::Down),
    #[command(name = "set")]
    Set(set::Set),
}
