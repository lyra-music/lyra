use lyra_proc::{BotGuildAutocompleteGroup, BotGuildCommandGroup};
use twilight_interactions::command::{CommandModel, CreateCommand};

pub mod backward;
mod first;
mod forward;
pub mod to;

#[derive(CommandModel, CreateCommand, BotGuildCommandGroup)]
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

#[derive(CommandModel, BotGuildAutocompleteGroup)]
#[command(autocomplete = true)]
pub enum Autocomplete {
    #[command(name = "to")]
    To(to::Autocomplete),
}
