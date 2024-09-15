use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};

use crate::{
    command::{
        check::{self, ResolveWithPoll, StartPoll},
        macros::out_or_upd,
        model::{BotSlashCommand, CtxKind, GuildCtx, RespondViaMessage},
        poll::Topic,
        require,
        util::controller_fmt,
        SlashCtx,
    },
    error::{component::queue::RepeatError, CommandResult},
    lavalink::{Event, OwnedPlayerData, RepeatMode as LavalinkRepeatMode},
    LavalinkAndGuildIdAware,
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

impl From<RepeatMode> for LavalinkRepeatMode {
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
        let mode = {
            if let Some(mode) = self.mode {
                mode.into()
            } else {
                get_next_repeat_mode(&ctx).await
            }
        };

        let in_voice = require::in_voice(&ctx)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        require::queue_not_empty(&data_r)?;
        drop(data_r);

        check::user_in(in_voice)?
            .only()
            .or_else_try_resolve_with(Topic::Repeat(mode))?
            .and_then_start(&mut ctx)
            .await?;

        Ok(repeat(&mut ctx, data, mode, false).await?)
    }
}

pub async fn get_next_repeat_mode(ctx: &GuildCtx<impl CtxKind>) -> LavalinkRepeatMode {
    let mode = match ctx.get_player_data() {
        Some(data) => data.read().await.queue().repeat_mode(),
        None => LavalinkRepeatMode::Off,
    };
    mode.next()
}

pub async fn repeat(
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    data: OwnedPlayerData,
    mode: LavalinkRepeatMode,
    via_controller: bool,
) -> Result<(), RepeatError> {
    ctx.try_get_connection()?.dispatch(Event::QueueRepeat);
    data.write().await.queue_mut().set_repeat_mode(mode);
    let message = format!("{} {}", mode.emoji(), mode);
    let content = controller_fmt(ctx, via_controller, &message);
    out_or_upd!(content, ctx);
}
