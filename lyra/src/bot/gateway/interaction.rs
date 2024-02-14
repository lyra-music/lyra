use std::sync::Arc;

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
use crate::bot::{
    command::{
        macros::{bad, cant, caut, crit, err, hid, hid_fol, nope, note, out_upd, sus, sus_fol},
        model::{AutocompleteCtx, CommandInfoAware, Ctx, MessageCommand, SlashCommand},
        util::MessageLinkAware,
    },
    component::{connection::Join, queue::Play},
    core::{
        model::{
            BotState, InteractionInterface, OwnedBotState, UnitFollowupResult, UnitRespondResult,
        },
        r#const::exit_code::{DUBIOUS, FORBIDDEN, WARNING},
    },
    error::{
        command::{
            check::{
                AlternateVoteResponse, AnotherPollOngoingError, PollLossError, PollLossErrorKind,
            },
            declare::{CommandExecuteError, Fuunacee},
            util::{AutoJoinSuppressedError, ConfirmationError},
            Error as CommandError, Fe, RespondError,
        },
        gateway::{MatchConfirmationError, ProcessError, ProcessResult},
        AutoJoinAttemptFailed as AutoJoinAttemptFailedError, EPrint,
        PositionOutOfRange as PositionOutOfRangeError, Suppressed as SuppressedError,
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
            InteractionType::ModalSubmit | InteractionType::Ping => Ok(()),
            _ => unimplemented!(),
        }
    }
}

impl Context {
    async fn process_as_app_command(mut self) -> ProcessResult {
        let bot = self.bot;
        let i = bot.interaction().await?.interfaces(&self.inner);

        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            unreachable!()
        };

        let name = data.name.clone().into();

        let result = match data.kind {
            CommandType::ChatInput => {
                <Ctx<SlashCommand>>::from_partial_data(
                    self.inner,
                    &data,
                    bot.clone(),
                    self.latency,
                    self.sender,
                )
                .execute(*data)
                .await
            }
            CommandType::User => todo!(),
            CommandType::Message => {
                <Ctx<MessageCommand>>::from_partial_data(
                    self.inner,
                    &data,
                    bot.clone(),
                    self.latency,
                    self.sender,
                )
                .execute(*data)
                .await
            }
            _ => unimplemented!(),
        };

        let Err(source) = result else {
            return Ok(());
        };

        match source.flatten_until_user_not_allowed_as() {
            Fuunacee::UserNotAllowed(_) => {
                nope!("You are not allowed to use commands in this context.", i);
            }
            Fuunacee::Command(_) => {
                let CommandExecuteError::Command(error) = source else {
                    unreachable!()
                };
                match_error(error, name, i, &bot).await
            }
            _ => {
                crit!(format!(
                    "Something unexpectedly went wrong: ```rs\n{source:#?}``` Please report this to the bot developers."
                ), ?i);
                Err(ProcessError::CommandExecute { name, source })
            }
        }
    }

    async fn process_as_autocomplete(mut self) -> ProcessResult {
        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            unreachable!()
        };

        let name = data.name.clone().into();
        let Err(source) = <AutocompleteCtx>::from_partial_data(
            self.inner,
            &data,
            self.bot,
            self.latency,
            self.sender,
        )
        .execute(*data)
        .await
        else {
            return Ok(());
        };

        Err(ProcessError::AutocompleteExecute { name, source })
    }

    #[allow(clippy::unused_async)]
    async fn process_as_component(self) -> ProcessResult {
        // TODO: implement controller
        Ok(())
    }
}

async fn match_error(
    error: CommandError,
    command_name: Box<str>,
    i: InteractionInterface<'_>,
    bot: &BotState,
) -> Result<(), ProcessError> {
    match error.flatten_as() {
        Fe::Cache(_) => {
            tracing::warn!("cache error: {:#?}", error);

            crit!("Something isn't working at the moment, try again later.", i);
        }
        Fe::UserNotDj(_) => {
            nope!("You need to be a ***DJ*** to do that.", i);
        }
        Fe::UserNotAccessManager(_) => {
            nope!("You need to be an ***Access Manager*** to do that.", i);
        }
        // Fe::UserNotPlaylistManager(_) => {
        //     nope!("You need to be a ***Playlist Manager*** to do that.", i);
        // }
        Fe::NotInVoice(_) => {
            let inter = bot.interaction().await?;
            let join = inter.mention_global_command(Join::name()).await?;
            let play = inter.mention_global_command(Play::name()).await?;
            caut!(
                format!(
                    "Not currently connected to a voice channel. Use {} or {} first.",
                    join, play
                ),
                i
            );
        }
        Fe::InVoiceWithoutUser(e) => {
            nope!(
                format!(
                    "You are not with the bot in {}; You need to be a ***DJ*** to do that.",
                    e.0.mention(),
                ),
                i
            );
        }
        Fe::InVoiceWithSomeoneElse(e) => {
            nope!(e.eprint(), i);
        }
        Fe::InVoiceWithoutSomeoneElse(e) => {
            bad!(format!("Not enough people are in {}.", e.0.mention()), i);
        }
        Fe::Suppressed(e) => Ok(match_suppressed(e, i).await?),
        Fe::AutoJoinSuppressed(e) => Ok(match_autojoin_suppressed(e, i).await?),
        Fe::AutoJoinAttemptFailed(e) => Ok(match_autojoin_attempt_failed(e, i, bot).await?),
        Fe::Stopped(_) => todo!(),
        Fe::NotPlaying(_) => todo!(),
        Fe::Paused(_) => todo!(),
        Fe::QueueNotSeekable(e) => {
            nope!(e.eprint(), i);
        }
        Fe::QueueEmpty(_) => {
            bad!("The queue is currently empty.", i);
        }
        Fe::PositionOutOfRange(e) => Ok(match_position_out_of_range(e, i).await?),
        Fe::NotUsersTrack(e) => {
            nope!(e.eprint(), i);
        }
        Fe::AnotherPollOngoing(e) => Ok(match_another_poll_ongoing(e, i).await?),
        Fe::PollLoss(e) => Ok(match_poll_loss(e, i).await?),
        Fe::PollVoided(e) => {
            out_upd!(
                format!("{WARNING} This poll has been voided as: {}.", e.eprint()),
                i
            );
        }
        Fe::Confirmation(e) => Ok(match_confirmation(e, i).await?),
        _ => {
            err!(format!("Something went wrong: ```rs\n{error:#?}```"), ?i);
            Err(ProcessError::CommandExecute {
                name: command_name,
                source: error.into(),
            })
        }
    }
}

async fn match_suppressed(
    error: &SuppressedError,
    i: InteractionInterface<'_>,
) -> UnitRespondResult {
    match error {
        SuppressedError::Muted => {
            bad!("Currently server muted.", i);
        }
        SuppressedError::NotSpeaker => {
            bad!("Not currently a speaker in this stage channel.", i);
        }
    }
}

async fn match_autojoin_suppressed(
    error: &AutoJoinSuppressedError,
    i: InteractionInterface<'_>,
) -> UnitFollowupResult {
    match error {
        AutoJoinSuppressedError::Muted => {
            sus_fol!("Can't use this command as is currently server muted.", i);
        }
        AutoJoinSuppressedError::StillNotSpeaker { last_followup_id } => {
            i.update_followup(
                *last_followup_id,
                &format!(
                    "{DUBIOUS} Timed out waiting to become speaker. Inform stage moderators to invite to speak and reinvoke this command."
                )
            ).await?;
            Ok(())
        }
    }
}

async fn match_autojoin_attempt_failed(
    error: &AutoJoinAttemptFailedError,
    i: InteractionInterface<'_>,
    bot: &BotState,
) -> Result<(), RespondError> {
    match error {
        AutoJoinAttemptFailedError::UserNotInVoice(_) => {
            let join = bot
                .interaction()
                .await?
                .mention_global_command(Join::name())
                .await?;
            bad!(
                format!(
                    "Please join a voice channel, or use {} to connect to a channel.",
                    join
                ),
                i
            );
        }
        AutoJoinAttemptFailedError::UserNotAllowed(_) => {
            nope!("Attempting to join your currently connected channel failed as you are not allowed to use the bot here.", i);
        }
        AutoJoinAttemptFailedError::Forbidden(e) => {
            cant!(
                format!(
                    "Attempting to join {} failed due to insufficient permissions.",
                    e.0.mention()
                ),
                i
            );
        }
        AutoJoinAttemptFailedError::UserNotStageManager(_) => {
            nope!("Attempting to join your currently connected stage failed as you are not a **Stage Manager**.", i);
        }
    }
}

async fn match_position_out_of_range(
    error: &PositionOutOfRangeError,
    i: InteractionInterface<'_>,
) -> UnitRespondResult {
    let message = match error {
        PositionOutOfRangeError::OutOfRange {
            position,
            queue_len,
        } => {
            format!(
                "Invalid track position: `{position}`; Track position must be from `1` to `{queue_len}`."
            )
        }
        PositionOutOfRangeError::OnlyTrack(p) => {
            format!(
                "Invalid track position: `{p}`; Track position must be `1` as the queue currently only has one track."
            )
        }
    };

    bad!(message, i);
}

async fn match_confirmation(
    error: &ConfirmationError,
    i: InteractionInterface<'_>,
) -> Result<(), MatchConfirmationError> {
    match error {
        ConfirmationError::Cancelled => {
            note!("Cancelled executing this command.", i);
        }
        ConfirmationError::TimedOut => {
            sus_fol!("Confirmation timed out.", i);
        }
    }
}

async fn match_another_poll_ongoing(
    error: &AnotherPollOngoingError,
    i: InteractionInterface<'_>,
) -> UnitRespondResult {
    let message_link = error.message.link();

    match error.alternate_vote {
        Some(AlternateVoteResponse::Casted) => {
            note!(format!("The ongoing poll at {message_link} may resolve this. Your vote has automatically been casted."), i);
        }
        Some(AlternateVoteResponse::DjCasted) => {
            hid!("Superseded the ongoing poll at {message_link} to win.", i);
        }
        Some(AlternateVoteResponse::CastDenied) => {
            nope!(
                format!("The ongoing poll at {message_link} may resolve this, although you are not eligible to cast a vote there."),
                i
            );
        }
        Some(AlternateVoteResponse::CastedAlready(casted)) => {
            caut!(
                format!("The ongoing poll at {message_link} may resolve this, although you've already casted a vote: **{casted}**."),
                i
            );
        }
        None => {
            sus!(format!("Another poll is needed to resolve that. Please resolve the ongoing poll at {message_link} first."), i);
        }
    }
}

async fn match_poll_loss(error: &PollLossError, i: InteractionInterface<'_>) -> UnitFollowupResult {
    let PollLossError { source, kind } = error;

    let source_txt = match kind {
        PollLossErrorKind::UnanimousLoss => "",
        PollLossErrorKind::TimedOut => "Poll timed out: ",
        PollLossErrorKind::SupersededLossViaDj => "The poll was superseded to lose by a DJ: ",
    };

    out_upd!(format!("{FORBIDDEN} {source_txt}{}", source.eprint()), i);
}
