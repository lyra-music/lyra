use lyra_proc::{BotAutocompleteGroup, BotCommandGroup};
use twilight_interactions::command::{CommandModel, CreateCommand};

pub mod backward;
mod first;
mod forward;
pub mod to;

#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "jump", desc = ".", contexts = "guild")]
pub enum Jump {
    #[command(name = "to")]
    To(to::To),
    #[command(name = "forward")]
    Forward(forward::Forward),
    #[command(name = "backward")]
    Backward(backward::Backward),
    #[command(name = "first")]
    First(first::First),
}

#[derive(CommandModel, BotAutocompleteGroup)]
#[command(autocomplete = true)]
pub enum Autocomplete {
    #[command(name = "to")]
    To(to::Autocomplete),
}
