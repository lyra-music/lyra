mod custom;
mod off;
mod preset;

use lavalink_rs::model::player::{Equalizer, Filters};
use lyra_ext::num::usize_as_u8;
use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

const EQUALISER_N: usize = 15;

struct SetEqualiser([Equalizer; EQUALISER_N]);

impl SetEqualiser {
    const DEFAULT_GAIN: f64 = 0.0;

    fn new(equaliser: [Option<f64>; EQUALISER_N]) -> Option<Self> {
        const ERR_MARGIN: f64 = f64::EPSILON;

        let equaliser =
            equaliser.map(|o| o.filter(|o| (o - Self::DEFAULT_GAIN).abs() > ERR_MARGIN));
        equaliser.iter().any(Option::is_some).then(|| {
            Self(core::array::from_fn(|i| Equalizer {
                band: usize_as_u8(i),
                gain: equaliser[i].unwrap_or(Self::DEFAULT_GAIN),
            }))
        })
    }
}

impl super::ApplyFilter for Option<SetEqualiser> {
    fn apply_to(self, filter: Filters) -> Filters {
        Filters {
            equalizer: self.map(|f| f.0.into()),
            ..filter
        }
    }
}

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "equaliser", desc = ".", contexts = "guild")]
pub enum Equaliser {
    #[command(name = "preset")]
    Preset(preset::Preset),
    #[command(name = "custom")]
    Custom(Box<custom::Custom>),
    #[command(name = "off")]
    Off(off::Off),
}
