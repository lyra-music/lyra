use lavalink_rs::model::player::{Filters, Timescale};
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{bad, out},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    component::tuning::{
        check_user_is_dj_and_require_unsuppressed_player, ApplyFilter, UpdateFilter,
    },
    error::CommandResult,
};

use super::Tier;

struct SetPitch(Option<f64>);

impl SetPitch {
    const DEFAULT_PITCH: f64 = 1.;

    fn new(multiplier: f64) -> Option<Self> {
        const ERR_MARGIN: f64 = f64::EPSILON;

        (multiplier != 0.).then(|| {
            let m = ((multiplier - Self::DEFAULT_PITCH).abs() > ERR_MARGIN).then_some(multiplier);
            Self(m)
        })
    }

    const fn into_timescale_via(self, timescale: &Timescale) -> Timescale {
        Timescale {
            pitch: self.0,
            ..*timescale
        }
    }

    fn multiplier(&self) -> f64 {
        self.0.unwrap_or(Self::DEFAULT_PITCH)
    }

    fn tier(&self) -> Tier {
        match self.0 {
            None => Tier::Default,
            Some(0.0..=1.0) => Tier::Low,
            _ => Tier::High,
        }
    }
}

impl ApplyFilter for SetPitch {
    fn apply_to(self, filter: Filters) -> Filters {
        let timescale = Some(self.into_timescale_via(&filter.timescale.unwrap_or_default()));

        Filters {
            timescale,
            ..filter
        }
    }
}

/// Sets the playback pitch
#[derive(CommandModel, CreateCommand)]
#[command(name = "set")]
pub struct Set {
    /// Set the playback pitch with what multiplier? [Default: 1.0]
    #[command(min_value = 0)]
    multiplier: f64,
}

impl BotSlashCommand for Set {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let Some(update) = SetPitch::new(self.multiplier) else {
            bad!("Multiplier must not be 0", ctx);
        };

        let multiplier = update.multiplier();
        let emoji = update.tier().emoji();
        player.update_filter(update).await?;
        player.data().write().await.pitch_mut().set(multiplier);

        out!(
            format!("{emoji} Set the playback pitch to `{multiplier}`×."),
            ctx
        );
    }
}
