use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(CreateCommand, CommandModel)]
#[command(name = "vroom", desc = "Vroom!")]
pub struct Vroom;
