mod all_off;
mod channel_mix;
mod distortion;
mod low_pass;
mod pitch;
mod rotation;
mod tremolo;
mod vibrato;

use std::marker::PhantomData;

use lavalink_rs::model::player::{Filters, TremoloVibrato};
use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

use super::ApplyFilter;

trait TremoloVibratoMarker {}
struct TremoloMarker;
impl TremoloVibratoMarker for TremoloMarker {}
struct VibratoMarker;
impl TremoloVibratoMarker for VibratoMarker {}
struct SetTremoloVibrato<T>
where
    T: TremoloVibratoMarker,
{
    inner: TremoloVibrato,
    kind: PhantomData<T>,
}

impl<T> SetTremoloVibrato<T>
where
    T: TremoloVibratoMarker,
{
    const SANE_DEFAULT_FREQUENCY: f64 = 2.;
    const SANE_DEFAULT_DEPTH: f64 = 0.5;

    fn new(frequency: Option<f64>, depth: Option<f64>) -> Option<Self> {
        ((frequency, depth) != (Some(0.), Some(0.))).then_some({
            let inner = TremoloVibrato { frequency, depth };
            Self {
                inner,
                kind: PhantomData,
            }
        })
    }

    fn settings(&self) -> TremoloVibratoSettings {
        const ERR_MARGIN: f64 = f64::EPSILON;

        let frequency = self
            .inner
            .frequency
            .filter(|f| (f - Self::SANE_DEFAULT_FREQUENCY).abs() > ERR_MARGIN);
        let depth = self
            .inner
            .depth
            .filter(|d| (d - Self::SANE_DEFAULT_DEPTH).abs() > ERR_MARGIN);

        match (frequency, depth) {
            (None, None) => TremoloVibratoSettings::Default,
            (None, Some(d)) => TremoloVibratoSettings::Depth(d),
            (Some(f), None) => TremoloVibratoSettings::Frequency(f),
            (Some(frequency), Some(depth)) => TremoloVibratoSettings::Custom { frequency, depth },
        }
    }
}

enum TremoloVibratoSettings {
    Default,
    Frequency(f64),
    Depth(f64),
    Custom { frequency: f64, depth: f64 },
}

impl std::fmt::Display for TremoloVibratoSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => f.write_str("**`Default Settings`**"),
            Self::Depth(d) => write!(f, "Depth: `{d}`"),
            Self::Frequency(frequency) => write!(f, "Frequency: `{frequency} Hz.`"),
            Self::Custom { frequency, depth } => {
                write!(f, "Frequency: `{frequency} Hz.`, Depth: `{depth}`")
            }
        }
    }
}

type SetTremolo = SetTremoloVibrato<TremoloMarker>;
type SetVibrato = SetTremoloVibrato<VibratoMarker>;

impl ApplyFilter for Option<SetTremoloVibrato<TremoloMarker>> {
    fn apply_to(self, filter: Filters) -> Filters {
        Filters {
            tremolo: self.map(|f| f.inner),
            ..filter
        }
    }
}

impl ApplyFilter for Option<SetTremoloVibrato<VibratoMarker>> {
    fn apply_to(self, filter: Filters) -> Filters {
        Filters {
            vibrato: self.map(|f| f.inner),
            ..filter
        }
    }
}

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "filter", desc = ".", contexts = "guild")]
pub enum Filter {
    #[command(name = "tremolo")]
    Tremolo(tremolo::Tremolo),
    #[command(name = "vibrato")]
    Vibrato(vibrato::Vibrato),
    #[command(name = "rotation")]
    Rotation(rotation::Rotation),
    #[command(name = "distortion")]
    Distortion(distortion::Distortion),
    #[command(name = "channel-mix")]
    ChannelMix(channel_mix::ChannelMix),
    #[command(name = "low-pass")]
    LowPass(low_pass::LowPass),
    #[command(name = "pitch")]
    Pitch(pitch::Pitch),
    #[command(name = "all-off")]
    AllOff(all_off::AllOff),
}
