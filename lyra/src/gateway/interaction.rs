use std::{hint::unreachable_unchecked, sync::Arc};

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
    command::{
        macros::{
            bad, bad_or_fol, cant_or_fol, caut, crit, crit_or_fol, err, hid, nope, nope_or_fol,
            note, out_upd, sus, sus_fol,
        },
        model::NonPingInteraction,
        require,
        util::MessageLinkAware,
        AutocompleteCtx, MessageCtx, SlashCtx,
    },
    component::{connection::Join, queue::Play},
    core::{
        model::{
            BotState, InteractionClient, InteractionInterface, OwnedBotState, UnitFollowupResult,
            UnitRespondResult,
        },
        r#const::exit_code::{DUBIOUS, PROHIBITED, WARNING},
    },
    error::{
        command::{
            check::{
                AlternateVoteResponse, AnotherPollOngoingError, PollLossError, PollLossErrorKind,
            },
            declare::{CommandExecuteError, Fuunacee},
            util::AutoJoinSuppressedError,
            Error as CommandError, Fe,
        },
        gateway::{ProcessError, ProcessResult},
        AutoJoinAttemptFailed as AutoJoinAttemptFailedError,
        PositionOutOfRange as PositionOutOfRangeError, PrettyErrorDisplay,
        Suppressed as SuppressedError,
    },
    LavalinkAware,
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
            // SAFETY: `self.inner.kind` is `ApplicationCommand`, so this is safe
            InteractionType::ApplicationCommand => unsafe { self.process_as_app_command() }.await,
            InteractionType::ApplicationCommandAutocomplete => {
                // SAFETY: `self.inner.kind` is `ApplicationCommandAutocomplete`, so this is safe
                unsafe { self.process_as_autocomplete() }.await
            }
            // SAFETY: `self.inner.kind` is `MessageComponent`, so this is safe
            InteractionType::MessageComponent => unsafe { self.process_as_component() }.await,
            // SAFETY: `self.inner.kind` is `ModalSubmit`, so this is safe
            InteractionType::ModalSubmit => unsafe { self.process_as_modal() }.await,
            InteractionType::Ping => Ok(()), // ignored
            _ => unimplemented!(),
        }
    }
}

impl Context {
    async unsafe fn process_as_app_command(mut self) -> ProcessResult {
        let bot = self.bot;
        let i = bot.interaction().await?.interfaces(&self.inner);

        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            // SAFETY: interaction is of type `ApplicationCommand`,
            //         so `self.inner.data.take()` will always be `InteractionData::ApplicationCommand(_)`
            unsafe { std::hint::unreachable_unchecked() }
        };

        let name = data.name.clone().into();
        let inner_guild_id = self.inner.guild_id;
        // SAFETY: interaction type is not `Ping`, so `channel` is present
        let channel_id = unsafe { self.inner.channel_id_unchecked() };
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
            if let Some(mut connection) = lavalink.get_connection_mut(guild_id) {
                connection.text_channel_id = channel_id;
            }
            if let Ok(player) = require::player(&(lavalink, guild_id)) {
                player.data().write().await.set_text_channel_id(channel_id);
            }
        }

        let Err(source) = result else {
            return Ok(());
        };

        let acknowledged = rx.try_recv().is_ok();
        match source.flatten_until_user_not_allowed_as() {
            Fuunacee::UserNotAllowed => {
                nope!("You are not allowed to use commands in this context.", i);
            }
            Fuunacee::Command => {
                let CommandExecuteError::Command(error) = source else {
                    // SAFETY: `source.flatten_until_user_not_allowed_as()` is `Fuunacee::Command(_)`,
                    //         so the unflattened source error must be `CommandExecuteError::Command(_)`
                    unsafe { unreachable_unchecked() }
                };
                match_error(error, name, acknowledged, i).await
            }
            _ => {
                crit!(format!(
                    "Something unexpectedly went wrong: ```rs\n{source:#?}``` Please report this to the bot developers."
                ), ?i);
                Err(ProcessError::CommandExecute { name, source })
            }
        }
    }

    async unsafe fn process_as_autocomplete(mut self) -> ProcessResult {
        let Some(InteractionData::ApplicationCommand(data)) = self.inner.data.take() else {
            // SAFETY: interaction is of type `ApplicationCommandAutocomplete`,
            //         so `self.inner.data.take()` will always be `InteractionData::ApplicationCommand(_)`
            unsafe { unreachable_unchecked() }
        };

        let name = data.name.clone().into();
        let (tx, _) = oneshot::channel::<()>();
        let Err(source) = <AutocompleteCtx>::from_partial_data(
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

    #[allow(clippy::unused_async)]
    async unsafe fn process_as_component(mut self) -> ProcessResult {
        let Some(InteractionData::MessageComponent(data)) = self.inner.data.take() else {
            // SAFETY: interaction is of type `MessageComponent`,
            //         so `self.inner.data.take()` will always be `InteractionData::MessageComponent(_)`
            unsafe { unreachable_unchecked() }
        };
        tracing::trace!(?data);
        // TODO: implement controller

        Ok(())
    }

    #[allow(clippy::unused_async)]
    async unsafe fn process_as_modal(mut self) -> ProcessResult {
        let Some(InteractionData::ModalSubmit(data)) = self.inner.data.take() else {
            // SAFETY: interaction is of type `ModalSubmit`,
            //         so `self.inner.data.take()` will always be `InteractionData::ModalSubmit(_)`
            unsafe { unreachable_unchecked() }
        };
        tracing::trace!(?data);

        Ok(())
    }
}

async fn match_error(
    error: CommandError,
    command_name: Box<str>,
    acknowledged: bool,
    i: InteractionInterface<'_>,
) -> Result<(), ProcessError> {
    match error.flatten_as() {
        Fe::Cache => {
            tracing::warn!("cache error: {:#?}", error);

            crit_or_fol!(
                "Something isn't working at the moment, try again later.",
                (i, acknowledged)
            );
        }
        Fe::UserNotDj => {
            nope!("You need to be a ***DJ*** to do that.", i);
        }
        Fe::UserNotAccessManager => {
            nope!("You need to be an ***Access Manager*** to do that.", i);
        }
        // Fe::UserNotPlaylistManager(_) => {
        //     nope!("You need to be a ***Playlist Manager*** to do that.", i);
        // }
        Fe::NotInVoice => {
            let join = InteractionClient::mention_command::<Join>();
            let play = InteractionClient::mention_command::<Play>();
            caut!(
                format!(
                    "Not currently connected to a voice channel. Use {} or {} first.",
                    join, play
                ),
                i
            );
        }
        Fe::InVoiceWithoutUser(e) => {
            nope_or_fol!(
                format!(
                    "You are not with the bot in {}; You need to be a ***DJ*** to do that.",
                    e.0.mention(),
                ),
                (i, acknowledged)
            );
        }
        Fe::InVoiceWithSomeoneElse(e) => {
            nope!(e.pretty_display(), i);
        }
        Fe::InVoiceWithoutSomeoneElse(e) => {
            bad!(format!("Not enough people are in {}.", e.0.mention()), i);
        }
        Fe::Suppressed(e) => Ok(match_suppressed(e, (i, acknowledged)).await?),
        Fe::AutoJoinSuppressed(e) => Ok(match_autojoin_suppressed(e, i).await?),
        Fe::AutoJoinAttemptFailed(e) => {
            Ok(match_autojoin_attempt_failed(e, (i, acknowledged)).await?)
        }
        Fe::Stopped => todo!(),
        Fe::NotPlaying => {
            bad!("Currently not playing anything.", i);
        }
        Fe::Paused => {
            bad!("Currently paused.", i);
        }
        Fe::QueueNotSeekable(e) => {
            nope!(e.pretty_display(), i);
        }
        Fe::QueueEmpty => {
            bad!("The queue is currently empty.", i);
        }
        Fe::PositionOutOfRange(e) => Ok(match_position_out_of_range(e, i).await?),
        Fe::NotUsersTrack(e) => {
            nope!(e.pretty_display(), i);
        }
        Fe::AnotherPollOngoing(e) => Ok(match_another_poll_ongoing(e, i).await?),
        Fe::PollLoss(e) => Ok(match_poll_loss(e, i).await?),
        Fe::PollVoided(e) => {
            out_upd!(
                format!(
                    "{WARNING} This poll has been voided as: {}.",
                    e.pretty_display()
                ),
                i
            );
        }
        Fe::ConfirmationTimedOut => {
            sus_fol!("Confirmation timed out.", i);
        }
        Fe::NoPlayer => {
            let play = InteractionClient::mention_command::<Play>();
            caut!(format!("Not yet played anything. Use {} first.", play), i);
        }
        Fe::UnrecognisedConnection => {
            crit!(
                "The bot wasn't disconnected properly last session. Please wait for it to automatically leave the voice channel, then try again.",
                i
            );
        }
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
    mut ia: (InteractionInterface<'_>, bool),
) -> UnitFollowupResult {
    match error {
        SuppressedError::Muted => {
            bad_or_fol!("Currently server muted.", ia);
        }
        SuppressedError::NotSpeaker => {
            bad_or_fol!("Not currently a speaker in this stage channel.", ia);
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
    mut ia: (InteractionInterface<'_>, bool),
) -> UnitFollowupResult {
    match error {
        AutoJoinAttemptFailedError::UserNotInVoice(_) => {
            let join = InteractionClient::mention_command::<Join>();
            bad_or_fol!(
                format!(
                    "Please join a voice channel, or use {} to connect to a channel.",
                    join
                ),
                ia
            );
        }
        AutoJoinAttemptFailedError::UserNotAllowed(_) => {
            nope_or_fol!("Attempting to join your currently connected channel failed as you are not allowed to use the bot here.", ia);
        }
        AutoJoinAttemptFailedError::Forbidden(e) => {
            cant_or_fol!(
                format!(
                    "Attempting to join {} failed due to insufficient permissions.",
                    e.0.mention()
                ),
                ia
            );
        }
        AutoJoinAttemptFailedError::UserNotStageManager(_) => {
            nope_or_fol!("Attempting to join your currently connected stage failed as you are not a **Stage Manager**.", ia);
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

    out_upd!(
        format!("{PROHIBITED} {source_txt}{}", source.pretty_display()),
        i
    );
}
