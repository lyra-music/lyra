pub mod access;
pub mod now_playing;

use twilight_interactions::command::{CommandModel, CreateCommand};

use lyra_proc::BotGuildCommandGroup;

use self::{access::Access, now_playing::NowPlaying};

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "config", desc = ".", contexts = "guild")]
pub enum Config {
    #[command(name = "access")]
    Access(Box<Access>),
    #[command(name = "now-playing")]
    NowPlaying(NowPlaying),
}
