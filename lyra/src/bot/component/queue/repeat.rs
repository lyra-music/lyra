use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};

use crate::bot::{
    command::{
        check::CheckerBuilder,
        macros::out_or_upd,
        model::{BotSlashCommand, SlashCommand},
        poll::Topic,
        Ctx,
    },
    error::command::Result as CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{self, ClientAware},
};

#[derive(CommandOption, CreateOption)]
enum RepeatMode {
    #[option(name = "Off", value = 0)]
    Off,
    #[option(name = "All", value = 1)]
    All,
    #[option(name = "One", value = 2)]
    Track,
}

impl From<RepeatMode> for lavalink::RepeatMode {
    fn from(value: RepeatMode) -> Self {
        match value {
            RepeatMode::Off => Self::Off,
            RepeatMode::All => Self::All,
            RepeatMode::Track => Self::Track,
        }
    }
}

/// Sets a repeat mode of the queue
#[derive(CommandModel, CreateCommand)]
#[command(name = "repeat", dm_permission = false)]
pub struct Repeat {
    /// Which mode? (if not given, cycle between: All > One > Off)
    mode: Option<RepeatMode>,
}

impl BotSlashCommand for Repeat {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        let guild_id = ctx.guild_id_expected();
        let mode = self.mode.map_or_else(
            || {
                let mode = ctx
                    .lavalink()
                    .connections()
                    .get_mut(&guild_id)
                    .map_or(lavalink::RepeatMode::Off, |c| c.queue().repeat_mode());
                mode.next()
            },
            Into::into,
        );

        CheckerBuilder::new()
            .in_voice_with_user_only_with_poll(Topic::Repeat(mode))
            .queue_not_empty()
            .build()
            .run(&mut ctx)
            .await?;

        let lavalink = ctx.lavalink();
        lavalink.dispatch(guild_id, lavalink::Event::QueueRepeat);
        lavalink
            .connection_mut(guild_id)
            .queue_mut()
            .set_repeat_mode(mode);

        let txt = &format!("{} {}", mode.emoji(), mode);
        out_or_upd!(txt, ctx);
    }
}
