use lavalink_rs::model::player::{Filters, Timescale};
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::require, component::tuning::UpdateFilter,
    core::model::response::initial::message::create::RespondWithMessage,
};

struct ResetAllExceptSpeed;

impl ResetAllExceptSpeed {
    const fn into_timescale_via(timescale: &Timescale) -> Timescale {
        Timescale {
            pitch: None,
            ..*timescale
        }
    }
}

impl super::ApplyFilter for ResetAllExceptSpeed {
    fn apply_to(self, filter: Filters) -> Filters {
        let timescale = Some(Self::into_timescale_via(
            &filter.timescale.unwrap_or_default(),
        ));

        Filters {
            timescale,
            ..Default::default()
        }
    }
}

/// Disables all filters.
#[derive(CommandModel, CreateCommand)]
#[command(name = "all-off")]
pub struct AllOff;

impl crate::command::model::BotSlashCommand for AllOff {
    async fn run(self, ctx: crate::command::SlashCmdCtx) -> crate::error::command::Result {
        let mut ctx = require::guild(ctx)?;
        let player = require::player(&ctx)?;

        player.update_filter(ResetAllExceptSpeed).await?;
        player.data().write().await.pitch_mut().reset();

        ctx.out("🪄🔴 Disabled all filters.").await?;
        Ok(())
    }
}
