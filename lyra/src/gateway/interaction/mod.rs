pub mod app_command;
pub mod autocomplete;
pub mod component;
pub mod modal;

use std::{error::Error, sync::Arc};

use lavalink_rs::error::LavalinkError;
use lyra_ext::pretty::join::PrettyJoiner;
use twilight_gateway::{Latency, MessageSender};
use twilight_mention::Mention;
use twilight_model::{
    application::interaction::InteractionType, gateway::payload::incoming::InteractionCreate,
};

use super::model::Process;
use crate::{
    CommandError,
    command::common::PlaySource,
    component::{connection::Join, queue::Play},
    core::{
        http::InteractionClient,
        model::{
            BotState, OwnedBotState,
            ctx_head::CtxHead,
            response::{either::RespondOrFollowup, initial::message::create::RespondWithMessage},
        },
    },
    error::{
        InVoiceWithSomeoneElse, InVoiceWithoutUser, NotUsersTrack, PrettyErrorDisplay, Suppressed,
        command::FlattenedError,
        core::{RespondError, RespondOrFollowupError},
        gateway::{
            ProcessError, ProcessResult,
            component::{ControllerError, FlattenedControllerError},
        },
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

trait FlattenAsLavalink {
    fn flatten_as_lavalink(&self) -> &LavalinkError;
}

impl FlattenAsLavalink for ControllerError {
    fn flatten_as_lavalink(&self) -> &LavalinkError {
        let FlattenedControllerError::Lavalink(e) = self.flatten_as() else {
            unreachable!();
        };
        e
    }
}

impl FlattenAsLavalink for CommandError {
    fn flatten_as_lavalink(&self) -> &LavalinkError {
        let FlattenedError::Lavalink(e) = self.flatten_as() else {
            unreachable!();
        };
        e
    }
}

async fn match_lavalink<E, F>(error: E, g: impl FnOnce(E) -> F, i: &mut CtxHead) -> Result<(), F>
where
    E: Error + FlattenAsLavalink,
    F: Into<ProcessError> + From<RespondOrFollowupError>,
{
    if let LavalinkError::TrackError(_error) = error.flatten_as_lavalink() {
        i.hid_f(format!(
            // As of Lavalink v4 API, the `severity` doesn't mean much to the user,
            // and `message` is almost always "Something went wrong while looking up the track.",
            // with `cause` repeating the same message, so the information from the error object
            // is entirely ignored.
            "💔 **Unable to load track**: \
                    Please ensure the URL is from a supported audio streaming service and \
                    the content is publicly accessible.  \n\
                    -# **Supported streaming services**: {}. \
                    If you believe this should be loaded, contact the bot developers to report the issue.",
            PlaySource::display_names().pretty_join_with_and()
        ))
        .await?;
        Ok(())
    } else {
        i.erro_f(format!(
            "Something went wrong with lavalink: ```rs\n{error:#?}```"
        ))
        .await?;
        Err(g(error))
    }
}

async fn match_wildcard<E, F>(error: E, g: impl FnOnce(E) -> F, i: &mut CtxHead) -> Result<(), F>
where
    E: Error,
    F: Into<ProcessError> + From<RespondError>,
{
    i.erro(format!("Something went wrong: ```rs\n{error:#?}```"));
    Err(g(error))
}

async fn match_cache(error: impl Error, mut i: CtxHead) -> Result<(), RespondOrFollowupError> {
    tracing::warn!("cache error: {:#?}", error);

    i.unkn_f("Something isn't working at the moment, try again later.")
        .await?;
    Ok(())
}

async fn match_in_voice_without_user(
    error: &InVoiceWithoutUser,
    mut i: CtxHead,
) -> Result<(), RespondOrFollowupError> {
    i.nope_f(format!(
        "You are not with the bot in {}.\n\
            -# Members who are a ***DJ*** bypass this.",
        error.0.mention(),
    ))
    .await?;
    Ok(())
}

type UnitRespondResult = Result<(), RespondError>;
type UnitRespondOrFollowupResult = Result<(), RespondOrFollowupError>;

const SUPPRESSED_MESSAGE: &str = "Currently server muted.";

async fn match_suppressed(error: &Suppressed, mut i: CtxHead) -> UnitRespondOrFollowupResult {
    match error {
        Suppressed::Muted => {
            i.wrng_f(SUPPRESSED_MESSAGE).await?;
        }
        Suppressed::NotSpeaker => {
            i.wrng_f("Not currently a speaker in this stage channel.")
                .await?;
        }
    }
    Ok(())
}

async fn match_not_in_voice(mut i: CtxHead) -> UnitRespondResult {
    let join = InteractionClient::mention_command::<Join>();
    let play = InteractionClient::mention_command::<Play>();
    i.warn(format!(
        "Not currently connected to a voice channel. Use {join} or {play} first.",
    ))
    .await?;
    Ok(())
}

async fn match_in_voice_with_someone_else(
    error: &InVoiceWithSomeoneElse,
    mut i: CtxHead,
) -> UnitRespondResult {
    i.nope(error.pretty_display().to_string()).await?;
    Ok(())
}

async fn match_not_playing(mut i: CtxHead) -> UnitRespondResult {
    i.wrng("Currently not playing anything.").await?;
    Ok(())
}

async fn match_unrecognised_connection(mut i: CtxHead) -> UnitRespondResult {
    i.unkn(
        "The bot wasn't disconnected properly last session. \
        Please wait for it to automatically leave the voice channel, then try again.",
    )
    .await?;
    Ok(())
}

async fn match_not_users_track(error: &NotUsersTrack, mut i: CtxHead) -> UnitRespondResult {
    i.nope(error.pretty_display().to_string()).await?;
    Ok(())
}
