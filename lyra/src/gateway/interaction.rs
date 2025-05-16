use std::sync::Arc;

use lavalink_rs::error::LavalinkError;
use lyra_ext::pretty::flags_display::FlagsDisplay;
use tokio::sync::oneshot;
use twilight_gateway::{Latency, MessageSender};
use twilight_mention::Mention;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{InteractionData, InteractionType},
    },
    gateway::payload::incoming::InteractionCreate,
};

use super::model::Process;
use crate::{
    CommandError, LavalinkAware,
    command::{
        AutocompleteCtx, MessageCtx, SlashCtx,
        model::{ComponentCtx, NonPingInteraction},
        require,
        util::MessageLinkAware,
    },
    component::{connection::Join, queue::Play},
    core::{
        r#const::exit_code::DUBIOUS,
        http::InteractionClient,
        model::{
            BotState, OwnedBotState,
            ctx_head::CtxHead,
            response::{
                Respond, either::RespondOrFollowup, followup::Followup,
                initial::message::create::RespondWithMessage,
            },
        },
        r#static::component::NowPlayingButtonType,
    },
    error::{
        AutoJoinAttemptFailed as AutoJoinAttemptFailedError,
        PositionOutOfRange as PositionOutOfRangeError, PrettyErrorDisplay,
        Suppressed as SuppressedError,
        command::{
            Fe,
            check::{
                AlternateVoteResponse, AnotherPollOngoingError, PollLossError, PollLossErrorKind,
            },
            declare::{CommandExecuteError, Fuunacee},
            util::AutoJoinSuppressedError,
        },
        core::{RespondError, RespondOrFollowupError},
        gateway::{ProcessError, ProcessResult},
    },
};

pub(super) struct Context {
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
}

impl BotState {
    pub(super) const fn into_interaction_create_context(
        self: Arc<Self>,
        inner: Box<InteractionCreate>,
        latency: Latency,
        sender: MessageSender,
    ) -> Context {
        Context {
            inner,
            bot: self,
            sender,
            latency,
        }
    }
}

impl Process for Context {
    async fn process(self) -> ProcessResult {
        match self.inner.kind {
            InteractionType::ApplicationCommand => self.process_as_app_command().await,
            InteractionType::ApplicationCommandAutocomplete => self.process_as_autocomplete().await,
            InteractionType::MessageComponent => self.process_as_component().await,
            InteractionType::ModalSubmit => self.process_as_modal().await,
            InteractionType::Ping => Ok(()), // ignored
            _ => unimplemented!(),
        }
    }
}

impl Context {
    async fn process_as_app_command(mut self) -> ProcessResult {
        let bot = self.bot;
        let mut i = bot.interaction().ctx(&self.inner);

        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            unreachable!()
        };

        let name = data.name.clone().into();
        let inner_guild_id = self.inner.guild_id;
        let channel_id = self.inner.channel_id_expected();
        let (tx, mut rx) = oneshot::channel::<()>();

        let result = match data.kind {
            CommandType::ChatInput => {
                SlashCtx::from_partial_data(
                    self.inner,
                    &data,
                    bot.clone(),
                    self.latency,
                    self.sender,
                    tx,
                )
                .execute(*data)
                .await
            }
            CommandType::Message => {
                MessageCtx::from_partial_data(
                    self.inner,
                    &data,
                    bot.clone(),
                    self.latency,
                    self.sender,
                    tx,
                )
                .execute(*data)
                .await
            }
            CommandType::User => todo!(),
            _ => unimplemented!(),
        };

        if let Some(guild_id) = inner_guild_id {
            let lavalink = bot.lavalink();
            lavalink.handle_for(guild_id).set_text_channel(channel_id);
            if let Ok(player) = require::player(&(lavalink, guild_id)) {
                player.data().write().await.set_text_channel_id(channel_id);
            }
        }

        let Err(source) = result else {
            return Ok(());
        };

        if rx.try_recv().is_ok() {
            i.acknowledge();
        }
        match source.flatten_until_user_not_allowed_as() {
            Fuunacee::UserNotAllowed => {
                i.nope("You are not allowed to use commands in this context.")
                    .await?;
                Ok(())
            }
            Fuunacee::Command => {
                let CommandExecuteError::Command(error) = source else {
                    unreachable!()
                };
                match_error(error, name, i).await
            }
            _ => {
                i.unkn(format!(
                    "Something unexpectedly went wrong: ```rs\n{source:#?}```
                    Please report this to the bot developers."
                ))
                .await?;
                Err(ProcessError::CommandExecute { name, source })
            }
        }
    }

    async fn process_as_autocomplete(mut self) -> ProcessResult {
        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            unreachable!()
        };

        let name = data.name.clone().into();
        let (tx, _) = oneshot::channel::<()>();
        let Err(source) = AutocompleteCtx::from_partial_data(
            self.inner,
            &data,
            self.bot,
            self.latency,
            self.sender,
            tx,
        )
        .execute(*data)
        .await
        else {
            return Ok(());
        };

        Err(ProcessError::AutocompleteExecute { name, source })
    }

    async fn process_as_component(mut self) -> ProcessResult {
        let Some(InteractionData::MessageComponent(data)) = self.inner.data.take() else {
            unreachable!()
        };
        tracing::trace!(?data);

        let ctx = ComponentCtx::from_data(self.inner, data, self.bot, self.latency, self.sender);
        let Ok(mut ctx) = require::guild(ctx) else {
            return Ok(());
        };
        let Ok(player) = require::player(&ctx) else {
            return Ok(());
        };

        let player_data = player.data();
        let player_data_r = player_data.read().await;
        let now_playing_message_id = player_data_r.now_playing_message_id();
        if now_playing_message_id.is_none_or(|id| id != ctx.message().id) {
            return Ok(());
        }
        let Some(current_track_title) = player_data_r
            .queue()
            .current()
            .map(|item| item.data().info.title.clone())
        else {
            return Ok(());
        };
        drop(player_data_r);

        let Some(now_playing_button) = ctx.take_custom_id_into_now_playing_button_type() else {
            return Ok(());
        };
        match now_playing_button {
            NowPlayingButtonType::Shuffle => {
                crate::component::queue::shuffle(player_data.clone(), &mut ctx, true).await?;
            }
            NowPlayingButtonType::Previous => {
                crate::component::playback::back(
                    Some(current_track_title),
                    player,
                    player_data.clone(),
                    &mut ctx,
                    true,
                )
                .await?;
            }
            NowPlayingButtonType::PlayPause => {
                crate::component::playback::play_pause(player, player_data.clone(), &mut ctx, true)
                    .await?;
            }
            NowPlayingButtonType::Next => {
                crate::component::playback::skip(
                    &current_track_title,
                    player,
                    player_data.clone(),
                    &mut ctx,
                    true,
                )
                .await?;
            }
            NowPlayingButtonType::Repeat => {
                let mode = crate::component::queue::get_next_repeat_mode(&ctx).await;
                crate::component::queue::repeat(&mut ctx, player_data.clone(), mode, true).await?;
            }
        }

        Ok(())
    }

    #[expect(clippy::unused_async)]
    async fn process_as_modal(mut self) -> ProcessResult {
        let Some(InteractionData::ModalSubmit(data)) = self.inner.data.take() else {
            unreachable!()
        };
        tracing::trace!(?data);

        Ok(())
    }
}

type UnitRespondResult = Result<(), RespondError>;
type UnitRespondOrFollowupResult = Result<(), RespondOrFollowupError>;

#[expect(clippy::too_many_lines)]
async fn match_error(
    error: CommandError,
    command_name: Box<str>,
    mut i: CtxHead,
) -> Result<(), ProcessError> {
    match error.flatten_as() {
        //: possibly deferred from /play {{{
        //:
        Fe::Cache => {
            tracing::warn!("cache error: {:#?}", error);

            i.unkn_f("Something isn't working at the moment, try again later.")
                .await?;
            Ok(())
        }
        Fe::InVoiceWithoutUser(e) => {
            i.nope_f(format!(
                "You are not with the bot in {}.\n\
                    -# Members who are a ***DJ*** bypass this.",
                e.0.mention(),
            ))
            .await?;
            Ok(())
        }
        Fe::Suppressed(e) => Ok(match_suppressed(e, i).await?),
        Fe::AutoJoinSuppressed(e) => Ok(match_autojoin_suppressed(e, i).await?),
        Fe::AutoJoinAttemptFailed(e) => Ok(match_autojoin_attempt_failed(e, i).await?),
        Fe::Lavalink(e) => {
            if let LavalinkError::TrackError(e) = e {
                i.hid_f(format!("💔 Error loading this track: {}", e.message))
                    .await?;
                Ok(())
            } else {
                i.erro_f(format!(
                    "Something went wrong with lavalink: ```rs\n{error:#?}```"
                ))
                .await?;
                Err(ProcessError::CommandExecute {
                    name: command_name,
                    source: error.into(),
                })
            }
        }
        Fe::TwilightHttp
        | Fe::DeserializeBody
        | Fe::MessageValidation
        | Fe::Sqlx
        | Fe::TaskJoin
        | Fe::GatewaySend => {
            i.erro_f(format!("Something went wrong: ```rs\n{error:#?}```"));
            Err(ProcessError::CommandExecute {
                name: command_name,
                source: error.into(),
            })
        }
        //:
        //: }}}
        Fe::UserNotDj => {
            i.nope("You need to be a ***DJ*** to do that.").await?;
            Ok(())
        }
        Fe::UserNotAccessManager => {
            i.nope("You need to be an ***Access Manager*** to do that.")
                .await?;
            Ok(())
        }
        // Fe::UserNotPlaylistManager(_) => {
        //     nope!("You need to be a ***Playlist Manager*** to do that.", i);
        // }
        Fe::NotInVoice => {
            let join = InteractionClient::mention_command::<Join>();
            let play = InteractionClient::mention_command::<Play>();
            i.warn(format!(
                "Not currently connected to a voice channel. Use {join} or {play} first.",
            ))
            .await?;
            Ok(())
        }
        Fe::InVoiceWithSomeoneElse(e) => {
            i.nope(e.pretty_display().to_string()).await?;
            Ok(())
        }
        Fe::InVoiceWithoutSomeoneElse(e) => {
            i.wrng(format!("Not enough people are in {}.", e.0.mention()))
                .await?;
            Ok(())
        }

        Fe::Stopped => todo!(),
        Fe::NotPlaying => {
            i.wrng("Currently not playing anything.").await?;
            Ok(())
        }
        Fe::Paused => {
            i.wrng("Currently paused.").await?;
            Ok(())
        }
        Fe::QueueNotSeekable(e) => {
            i.nope(e.pretty_display().to_string()).await?;
            Ok(())
        }
        Fe::QueueEmpty => {
            i.wrng("The queue is currently empty.").await?;
            Ok(())
        }
        Fe::PositionOutOfRange(e) => Ok(match_position_out_of_range(e, i).await?),
        Fe::NotUsersTrack(e) => {
            i.nope(e.pretty_display().to_string()).await?;
            Ok(())
        }
        Fe::AnotherPollOngoing(e) => Ok(match_another_poll_ongoing(e, i).await?),
        Fe::PollLoss(e) => Ok(match_poll_loss(e, i).await?),
        Fe::PollVoided(_e) => {
            //: TODO #44 {{{

            //out_upd!(
            //    format!(
            //        "{WARNING} This poll has been voided as: {}.",
            //        e.pretty_display()
            //    ),
            //    i
            //);
            todo!()
            //: }}}
        }
        Fe::ConfirmationTimedOut => {
            i.suspf("Confirmation timed out.").await?;
            Ok(())
        }
        Fe::NoPlayer => {
            let play = InteractionClient::mention_command::<Play>();
            i.warn(format!("Not yet played anything. Use {play} first."))
                .await?;
            Ok(())
        }
        Fe::UnrecognisedConnection => {
            i.unkn(
                "The bot wasn't disconnected properly last session. \
                Please wait for it to automatically leave the voice channel, then try again.",
            )
            .await?;
            Ok(())
        }
        _ => {
            i.erro(format!("Something went wrong: ```rs\n{error:#?}```"));
            Err(ProcessError::CommandExecute {
                name: command_name,
                source: error.into(),
            })
        }
    }
}

async fn match_suppressed(error: &SuppressedError, mut i: CtxHead) -> UnitRespondOrFollowupResult {
    match error {
        SuppressedError::Muted => {
            i.wrng_f("Currently server muted.").await?;
        }
        SuppressedError::NotSpeaker => {
            i.wrng_f("Not currently a speaker in this stage channel.")
                .await?;
        }
    }
    Ok(())
}

async fn match_autojoin_suppressed(
    error: &AutoJoinSuppressedError,
    i: CtxHead,
) -> UnitRespondResult {
    match error {
        AutoJoinSuppressedError::Muted => {
            i.suspf("Can't use this command as is currently server muted.")
                .await?;
        }
        AutoJoinSuppressedError::StillNotSpeaker { last_followup_id } => {
            i.update_followup(*last_followup_id)
                .content(format!(
                    "{DUBIOUS} Timed out waiting to become speaker. \
                    Inform stage moderators to invite to speak and reinvoke this command."
                ))
                .await?;
        }
    }
    Ok(())
}

async fn match_autojoin_attempt_failed(
    error: &AutoJoinAttemptFailedError,
    mut i: CtxHead,
) -> UnitRespondOrFollowupResult {
    match error {
        AutoJoinAttemptFailedError::UserNotInVoice(_) => {
            let join = InteractionClient::mention_command::<Join>();
            i.wrng_f(format!(
                "Please join a voice channel, or use {join} to connect to a channel.",
            ))
            .await?;
        }
        AutoJoinAttemptFailedError::UserNotAllowed(_) => {
            i.nope_f(
                "Attempting to join your currently connected channel failed as \
                you are not allowed to use the bot here.",
            )
            .await?;
        }
        AutoJoinAttemptFailedError::Forbidden(e) => {
            i.blck_f(format!(
                "**Attempting to join {} failed due to insufficient permissions**: \
                Missing {} permissions.",
                e.channel_id.mention(),
                e.missing.pretty_display_code()
            ))
            .await?;
        }
        AutoJoinAttemptFailedError::UserNotStageModerator(_) => {
            i.nope_f(
                "Attempting to join your currently connected stage failed as \
                you are not a **Stage Manager**.",
            )
            .await?;
        }
    }
    Ok(())
}

async fn match_position_out_of_range(
    error: &PositionOutOfRangeError,
    mut i: CtxHead,
) -> UnitRespondResult {
    let message = match error {
        PositionOutOfRangeError::OutOfRange {
            position,
            queue_len,
        } => {
            format!(
                "Invalid track position: `{position}`; \
                Track position must be from `1` to `{queue_len}`."
            )
        }
        PositionOutOfRangeError::OnlyTrack(p) => {
            format!(
                "Invalid track position: `{p}`; \
                Track position must be `1` as the queue currently only has one track."
            )
        }
    };

    i.wrng(message).await?;
    Ok(())
}

async fn match_another_poll_ongoing(
    error: &AnotherPollOngoingError,
    mut i: CtxHead,
) -> UnitRespondResult {
    let message_link = error.message.link();

    match error.alternate_vote {
        Some(AlternateVoteResponse::Casted) => {
            i.note(format!(
                "The ongoing poll at {message_link} may resolve this. \
                Your vote has automatically been casted."
            ))
            .await?;
        }
        Some(AlternateVoteResponse::DjCasted) => {
            i.hid(format!(
                "Superseded the ongoing poll at {message_link} to win."
            ))
            .await?;
        }
        Some(AlternateVoteResponse::CastDenied) => {
            i.nope(format!(
                "The ongoing poll at {message_link} may resolve this, \
                although you are not eligible to cast a vote there."
            ))
            .await?;
        }
        Some(AlternateVoteResponse::CastedAlready(casted)) => {
            i.warn(format!(
                "The ongoing poll at {message_link} may resolve this, \
                although you've already casted a vote: **{casted}**."
            ))
            .await?;
        }
        None => {
            i.susp(format!(
                "Another poll is needed to resolve that. \
                Please resolve the ongoing poll at {message_link} first."
            ))
            .await?;
        }
    }
    Ok(())
}

#[expect(clippy::unused_async)]
async fn match_poll_loss(error: &PollLossError, _: CtxHead) -> UnitRespondResult {
    let PollLossError { source: _, kind } = error;

    let _source_txt = match kind {
        PollLossErrorKind::UnanimousLoss => "",
        PollLossErrorKind::TimedOut => "Poll timed out: ",
        PollLossErrorKind::SupersededLossViaDj => "The poll was superseded to lose by a DJ: ",
    };

    // TODO: #44
    //out_upd!(
    //    format!("{PROHIBITED} {source_txt}{}", source.pretty_display()),
    //    i
    //);
    todo!()
}
