use twilight_model::gateway::{
    payload::outgoing::UpdatePresence,
    presence::{Activity, ActivityType, MinimalActivity, Status},
};

use super::Context;

pub fn handle_ready(ctx: Context) -> anyhow::Result<()> {
    ctx.bot().sender().command(&UpdatePresence::new(
        [Activity::from(MinimalActivity {
            kind: ActivityType::Listening,
            name: "/play".into(),
            url: None,
        })],
        false,
        None,
        Status::Online,
    )?)?;

    Ok(())
}
