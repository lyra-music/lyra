pub mod access;
pub mod now_playing;

use twilight_interactions::command::{CommandModel, CreateCommand};

use lyra_proc::BotGuildCommandGroup;
use twilight_model::guild::Permissions;

use self::{access::Access, now_playing::NowPlaying};

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(
    name = "config",
    desc = ".",
    contexts = "guild",
    default_permissions = "Self::default_permissions"
)]
pub enum Config {
    #[command(name = "access")]
    Access(Box<Access>),
    #[command(name = "now-playing")]
    NowPlaying(NowPlaying),
}

impl Config {
    const fn default_permissions() -> Permissions {
        Permissions::MANAGE_GUILD
    }
}
