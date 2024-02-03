pub mod access;
pub mod now_playing;

use twilight_interactions::command::{CommandModel, CreateCommand};

use self::{access::Access, now_playing::NowPlaying};
use crate::bot::{
    command::model::{BotSlashCommand, CommandInfoAware, Ctx, SlashCommand},
    error::command::Result as CommandResult,
};
use lyra_proc::BotCommandGroup;

/// -
#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "config", dm_permission = false)]
pub enum Config {
    #[command(name = "access")]
    Access(Box<Access>),
    #[command(name = "now-playing")]
    NowPlaying(NowPlaying),
}
