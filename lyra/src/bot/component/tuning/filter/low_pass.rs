use lavalink_rs::model::player::{Filters, LowPass as LavalinkLowPass};
use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{bad, out},
        model::BotSlashCommand,
        SlashCtx,
    },
    component::tuning::{common_checks, set_filter},
    error::CommandResult,
};

struct SetLowPass(LavalinkLowPass);

impl SetLowPass {
    const ERR_MARGIN: f64 = f64::EPSILON;
    const DEFAULT_SMOOTHING: f64 = 1.;
    const SANE_DEFAULT_SMOOTHING: f64 = 20.;

    fn new(smoothing: Option<f64>) -> Option<Self> {
        (!smoothing.is_some_and(|s| (s - Self::DEFAULT_SMOOTHING).abs() < Self::ERR_MARGIN))
            .then_some(Self(LavalinkLowPass { smoothing }))
    }

    fn settings(&self) -> LowPassSettings {
        let smoothing = self
            .0
            .smoothing
            .filter(|s| (s - Self::SANE_DEFAULT_SMOOTHING.abs() > Self::ERR_MARGIN));

        smoothing.map_or(LowPassSettings::Default, LowPassSettings::Custom)
    }
}

enum LowPassSettings {
    Default,
    Custom(f64),
}

impl std::fmt::Display for LowPassSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let settings = match self {
            Self::Default => String::from("**`Default Settings`**"),
            Self::Custom(s) => format!("Smoothing: `{s:.1}`"),
        };

        write!(f, "{settings}")
    }
}

impl crate::bot::component::tuning::UpdateFilter for Option<SetLowPass> {
    fn apply(self, filter: Filters) -> Filters {
        Filters {
            low_pass: self.map(|l| l.0),
            ..filter
        }
    }
}

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "low-pass", desc = ".")]
pub enum LowPass {
    #[command(name = "on")]
    On(On),
    #[command(name = "off")]
    Off(Off),
}

/// Enable Low Pass: Suppressing higher frequencies, making the audio "muffled".
#[derive(CommandModel, CreateCommand)]
#[command(name = "on")]
pub struct On {
    /// How much intensity for the low pass smoothing? (If not given, a reasonable default is used)
    #[command(min_value = 1)]
    smoothing: Option<f64>, // default: 20 [https://github.com/lavalink-devs/Lavalink/blob/master/protocol/src/commonMain/kotlin/dev/arbjerg/lavalink/protocol/v4/filters.kt#L120]
}

impl BotSlashCommand for On {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        common_checks(&ctx)?;

        let Some(update) = SetLowPass::new(self.smoothing) else {
            bad!(
                format!("Smoothing must not be `{}`.", SetLowPass::DEFAULT_SMOOTHING),
                ctx
            );
        };
        let settings = update.settings();

        set_filter(&ctx, Some(update)).await?;
        out!(format!("😶‍🌫️🟢 Enabled low pass ({settings})"), ctx);
    }
}

/// Disable Low Pass
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl BotSlashCommand for Off {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        common_checks(&ctx)?;

        set_filter(&ctx, None::<SetLowPass>).await?;
        out!("😶‍🌫️🔴 Disabled low pass", ctx);
    }
}
