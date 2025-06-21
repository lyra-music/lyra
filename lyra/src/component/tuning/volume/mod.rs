mod down;
mod set;
mod toggle_mute;
mod up;

use std::num::NonZeroU16;

use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::guild::Permissions;

pub(super) const fn volume_emoji(percent: Option<NonZeroU16>) -> &'static str {
    let Some(percent) = percent else {
        return "🔇";
    };
    match percent.get() {
        0 => unreachable!(),
        1..=33 => "🔈",
        34..=66 => "🔉",
        67..=100 => "🔊",
        101.. => "❕🔊",
    }
}

pub fn clipping_warning(percent: NonZeroU16) -> &'static str {
    (percent.get() > 100)
        .then_some(" (**`Audio quality may be reduced`**)")
        .unwrap_or_default()
}

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(
    name = "volume",
    desc = ".",
    contexts = "guild",
    default_permissions = "Self::default_permissions"
)]
pub enum Volume {
    #[command(name = "toggle-mute")]
    ToggleMute(toggle_mute::ToggleMute),
    #[command(name = "set")]
    Set(set::Set),
    #[command(name = "up")]
    Up(up::Up),
    #[command(name = "down")]
    Down(down::Down),
}

impl Volume {
    const fn default_permissions() -> Permissions {
        Permissions::MUTE_MEMBERS
    }
}
