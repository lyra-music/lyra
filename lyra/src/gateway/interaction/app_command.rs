use lyra_ext::pretty::flags_display::FlagsDisplay;
use tokio::sync::oneshot;
use twilight_mention::Mention;
use twilight_model::application::{command::CommandType, interaction::InteractionData};

use crate::{
    CommandError, LavalinkAware,
    command::{MessageCtx, SlashCtx, model::NonPingInteraction, require, util::MessageLinkAware},
    component::{connection::Join, queue::Play},
    core::{
        r#const::exit_code::DUBIOUS,
        http::InteractionClient,
        model::{
            ctx_head::CtxHead,
            response::{
                Respond, either::RespondOrFollowup, followup::Followup,
                initial::message::create::RespondWithMessage,
            },
        },
    },
    error::{
        AutoJoinAttemptFailed, PositionOutOfRange, PrettyErrorDisplay,
        command::{
            FlattenedError as Fe,
            check::{
                AlternateVoteResponse, AnotherPollOngoingError, PollLossError, PollLossErrorKind,
            },
            declare::{
                CommandExecuteError, FlattenedUntilUserNotAllowedCommandExecuteError as Fuunacee,
            },
            util::AutoJoinSuppressedError,
        },
        gateway::{ProcessError, ProcessResult},
    },
};

use super::{
    SUPPRESSED_MESSAGE, UnitRespondOrFollowupResult, UnitRespondResult, match_cache,
    match_in_voice_with_someone_else, match_in_voice_without_user, match_lavalink,
    match_not_in_voice, match_not_playing, match_not_users_track, match_suppressed,
    match_unrecognised_connection, match_wildcard,
};

impl super::Context {
    pub(super) async fn process_as_app_command(mut self) -> ProcessResult {
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
}

async fn match_error(
    error: CommandError,
    command_name: Box<str>,
    mut i: CtxHead,
) -> Result<(), ProcessError> {
    match error.flatten_as() {
        //: possibly deferred from /play {{{
        //:
        Fe::Cache => Ok(match_cache(error, i).await?),
        Fe::InVoiceWithoutUser(e) => Ok(match_in_voice_without_user(e, i).await?),
        Fe::Suppressed(e) => Ok(match_suppressed(e, i).await?),
        Fe::AutoJoinSuppressed(e) => Ok(match_autojoin_suppressed(e, i).await?),
        Fe::AutoJoinAttemptFailed(e) => Ok(match_autojoin_attempt_failed(e, i).await?),
        Fe::Lavalink(_) => {
            match_lavalink(
                error,
                move |e| ProcessError::CommandExecute {
                    name: command_name,
                    source: e.into(),
                },
                &mut i,
            )
            .await
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
        Fe::NotInVoice => Ok(match_not_in_voice(i).await?),
        Fe::InVoiceWithSomeoneElse(e) => Ok(match_in_voice_with_someone_else(e, i).await?),
        Fe::InVoiceWithoutSomeoneElse(e) => {
            i.wrng(format!("Not enough people are in {}.", e.0.mention()))
                .await?;
            Ok(())
        }

        Fe::Stopped => todo!(),
        Fe::NotPlaying => Ok(match_not_playing(i).await?),
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
        Fe::NotUsersTrack(e) => Ok(match_not_users_track(e, i).await?),
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
        Fe::UnrecognisedConnection => Ok(match_unrecognised_connection(i).await?),
        _ => Ok(match_wildcard(
            error,
            move |e| ProcessError::CommandExecute {
                name: command_name,
                source: e.into(),
            },
            &mut i,
        )
        .await?),
    }
}

async fn match_autojoin_suppressed(
    error: &AutoJoinSuppressedError,
    i: CtxHead,
) -> UnitRespondResult {
    match error {
        AutoJoinSuppressedError::Muted => {
            i.wrngf(SUPPRESSED_MESSAGE).await?;
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
    error: &AutoJoinAttemptFailed,
    mut i: CtxHead,
) -> UnitRespondOrFollowupResult {
    match error {
        AutoJoinAttemptFailed::UserNotInVoice(_) => {
            let join = InteractionClient::mention_command::<Join>();
            i.wrng_f(format!(
                "Please join a voice channel, or use {join} to connect to a channel.",
            ))
            .await?;
        }
        AutoJoinAttemptFailed::UserNotAllowed(_) => {
            i.nope_f(
                "Attempting to join your currently connected channel failed as \
                you are not allowed to use the bot here.",
            )
            .await?;
        }
        AutoJoinAttemptFailed::Forbidden(e) => {
            i.blck_f(format!(
                "**Attempting to join {} failed due to insufficient permissions**: \
                Missing {} permissions.",
                e.channel_id.mention(),
                e.missing.pretty_display_code()
            ))
            .await?;
        }
        AutoJoinAttemptFailed::UserNotStageModerator(_) => {
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
    error: &PositionOutOfRange,
    mut i: CtxHead,
) -> UnitRespondResult {
    let message = match error {
        PositionOutOfRange::OutOfRange {
            position,
            queue_len,
        } => {
            format!(
                "Invalid track position: `{position}`; \
                Track position must be from `1` to `{queue_len}`."
            )
        }
        PositionOutOfRange::OnlyTrack(p) => {
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
