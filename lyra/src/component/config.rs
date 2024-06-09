pub mod access;
pub mod now_playing;

use twilight_interactions::command::{CommandModel, CreateCommand};

use lyra_proc::BotCommandGroup;

use self::{access::Access, now_playing::NowPlaying};

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "config", desc = ".", dm_permission = false)]
pub enum Config {
    #[command(name = "access")]
    Access(Box<Access>),
    #[command(name = "now-playing")]
    NowPlaying(NowPlaying),
}
