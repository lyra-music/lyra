use lavalink_rs::model::player::{Filters, Timescale};
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{bad, out},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    component::tuning::{check_user_is_dj_and_require_unsuppressed_player, UpdateFilter},
    error::CommandResult,
};

use super::ApplyFilter;
enum Tier {
    Default,
    Fast,
    Slow,
}

impl Tier {
    const fn emoji(&self) -> &'static str {
        match self {
            Self::Default => "🚶",
            Self::Fast => "🐇",
            Self::Slow => "🐢",
        }
    }
}
struct SpeedFilter {
    multiplier: Option<f64>,
    pitch_shift: bool,
}

impl SpeedFilter {
    const DEFAULT_SPEED: f64 = 1.;

    fn new(multiplier: f64, pitch_shift: bool) -> Option<Self> {
        const ERR_MARGIN: f64 = f64::EPSILON;

        (multiplier != 0.).then(|| {
            let multiplier =
                ((multiplier - Self::DEFAULT_SPEED).abs() > ERR_MARGIN).then_some(multiplier);
            Self {
                multiplier,
                pitch_shift,
            }
        })
    }

    const fn into_timescale_via(self, timescale: &Timescale) -> Timescale {
        let pitch_shift = self.pitch_shift;
        let multiplier = self.multiplier;

        let (speed, rate) = if pitch_shift {
            (None, multiplier)
        } else {
            (multiplier, None)
        };

        Timescale {
            speed,
            rate,
            ..*timescale
        }
    }

    fn multiplier(&self) -> f64 {
        self.multiplier.unwrap_or(Self::DEFAULT_SPEED)
    }

    fn tier(&self) -> Tier {
        match self.multiplier {
            None => Tier::Default,
            Some(0.0..=1.0) => Tier::Slow,
            _ => Tier::Fast,
        }
    }
}

impl ApplyFilter for SpeedFilter {
    fn apply_to(self, filter: Filters) -> Filters {
        let timescale = Some(self.into_timescale_via(&filter.timescale.unwrap_or_default()));

        Filters {
            timescale,
            ..filter
        }
    }
}

/// Sets the playback speed
#[derive(CommandModel, CreateCommand)]
#[command(name = "speed", dm_permission = false)]
pub struct Speed {
    /// Sets the speed with what multiplier? [Default: 1.0]
    #[command(min_value = 0)]
    multiplier: f64,
    /// Also shifts the playback pitch? (If not given, no)
    pitch_shift: Option<bool>,
}

impl BotSlashCommand for Speed {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let Some(update) = SpeedFilter::new(self.multiplier, self.pitch_shift.unwrap_or_default())
        else {
            bad!("Multiplier must not be 0", ctx);
        };

        let multiplier = update.multiplier();
        let emoji = update.tier().emoji();
        player.update_filter(update).await?;

        out!(
            format!("{emoji} Set the playback speed to `{multiplier}`×."),
            ctx
        );
    }
}
