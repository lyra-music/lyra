use lavalink_rs::model::player::{Filters, Timescale};
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{command::macros::out, lavalink::ExpectedPlayerDataAware};

struct ResetAllExceptSpeed;

impl ResetAllExceptSpeed {
    const fn into_timescale_via(timescale: &Timescale) -> Timescale {
        Timescale {
            pitch: None,
            ..*timescale
        }
    }
}

impl super::UpdateFilter for ResetAllExceptSpeed {
    fn apply(self, filter: Filters) -> Filters {
        let timescale = Some(Self::into_timescale_via(
            &filter.timescale.unwrap_or_default(),
        ));

        Filters {
            timescale,
            ..Default::default()
        }
    }
}

/// Disable all filter
#[derive(CommandModel, CreateCommand)]
#[command(name = "all-off")]
pub struct AllOff;

impl crate::bot::command::model::BotSlashCommand for AllOff {
    async fn run(
        self,
        mut ctx: crate::bot::command::SlashCtx,
    ) -> crate::bot::error::command::Result {
        super::super::common_checks(&ctx)?;

        super::super::set_filter(&ctx, ResetAllExceptSpeed).await?;
        ctx.player_data().write().await.pitch_mut().reset();

        out!("🪄🔴 Disabled all filters", ctx);
    }
}
