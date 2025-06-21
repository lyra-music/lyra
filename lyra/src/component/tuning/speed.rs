use lavalink_rs::model::player::{Filters, Timescale};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::guild::Permissions;

use crate::{
    command::{
        check::DJ_PERMISSIONS,
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        require::PlayerInterface,
    },
    component::tuning::{UpdateFilter, require_in_voice_unsuppressed_and_player},
    core::model::response::initial::message::create::RespondWithMessage,
    error::{CommandResult, command::require::SetSpeedError},
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

    const fn multiplier(&self) -> f64 {
        match self.multiplier {
            Some(m) => m,
            None => Self::DEFAULT_SPEED,
        }
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

impl PlayerInterface {
    async fn set_speed(&self, update: SpeedFilter) -> Result<(), SetSpeedError> {
        let data = self.data();
        let mut data_w = data.write().await;
        let mul = update.multiplier();
        data_w.set_speed(mul);
        data_w.update_and_apply_now_playing_speed(mul).await?;
        drop(data_w);
        self.update_filter(update).await?;
        Ok(())
    }
}

/// Sets the playback speed.
#[derive(CommandModel, CreateCommand)]
#[command(
    name = "speed",
    contexts = "guild",
    default_permissions = "Self::default_permissions"
)]
pub struct Speed {
    /// Sets the speed with what multiplier? [Default: 1.0]
    #[command(min_value = 0)]
    multiplier: f64,
    /// Also shifts the playback pitch? (If not given, no)
    pitch_shift: Option<bool>,
}

impl Speed {
    const fn default_permissions() -> Permissions {
        DJ_PERMISSIONS
    }
}

impl BotGuildSlashCommand for Speed {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let Some(update) = SpeedFilter::new(self.multiplier, self.pitch_shift.unwrap_or_default())
        else {
            ctx.wrng("Multiplier must not be zero.").await?;
            return Ok(());
        };

        let multiplier = update.multiplier();
        let emoji = update.tier().emoji();
        player.set_speed(update).await?;

        ctx.out(format!(
            "{emoji} Set the playback speed to `{multiplier}`×."
        ))
        .await?;
        Ok(())
    }
}
