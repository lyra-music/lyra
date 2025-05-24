use lyra_proc::BotGuildCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

mod backward;
mod forward;
mod to;

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
#[command(name = "seek", desc = ".", contexts = "guild")]
pub enum Seek {
    #[command(name = "to")]
    To(to::To),
    #[command(name = "forward")]
    Forward(forward::Forward),
    #[command(name = "backward")]
    Backward(backward::Backward),
}
