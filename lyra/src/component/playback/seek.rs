use lyra_proc::BotCommandGroup;
use twilight_interactions::command::{CommandModel, CreateCommand};

mod backward;
mod forward;
mod to;

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "seek", desc = ".", dm_permission = false)]
pub enum Seek {
    #[command(name = "to")]
    To(to::To),
    #[command(name = "forward")]
    Forward(forward::Forward),
    #[command(name = "backward")]
    Backward(backward::Backward),
}
