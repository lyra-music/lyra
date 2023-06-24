pub mod access;
pub mod now_playing;

use anyhow::Result;
use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::id::{marker::CommandMarker, Id};

use self::{access::Access, now_playing::NowPlaying};
use crate::bot::commands::models::{App, Context, LyraCommand, ResolvedCommandInfo};
use lyra_proc::LyraCommandGroup;

#[derive(CommandModel, CreateCommand, LyraCommandGroup)]
#[command(name = "config", desc = ".", dm_permission = false)]
pub enum Config {
    #[command(name = "access")]
    Access(Box<Access>),
    #[command(name = "now-playing")]
    NowPlaying(NowPlaying),
}
