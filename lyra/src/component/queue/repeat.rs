use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};

use crate::{
    command::{
        check,
        macros::out_or_upd,
        model::BotSlashCommand,
        poll::Topic,
        require::{self, PartialInVoice},
        SlashCtx,
    },
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::{self, DelegateMethods, LavalinkAware},
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
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let guild_id = ctx.guild_id();
        let mode = {
            if let Some(mode) = self.mode {
                mode.into()
            } else {
                let mode = match ctx.lavalink().get_player_data(guild_id) {
                    Some(data) => data.write().await.queue().repeat_mode(),
                    None => lavalink::RepeatMode::Off,
                };
                mode.next()
            }
        };

        let in_voice = require::in_voice(&ctx)?;
        let in_voice_cacheless = PartialInVoice::from(&in_voice);
        let player = require::player(&ctx)?.and_queue_not_empty().await?;
        check::in_voice_with_user(in_voice)?
            .only_else_poll(Topic::Repeat(mode))?
            .start(&mut ctx)
            .await?;

        ctx.lavalink()
            .connection_from(&in_voice_cacheless)
            .dispatch(lavalink::Event::QueueRepeat);
        player
            .data()
            .write()
            .await
            .queue_mut()
            .set_repeat_mode(mode);

        let txt = &format!("{} {}", mode.emoji(), mode);
        out_or_upd!(txt, ctx);
    }
}
