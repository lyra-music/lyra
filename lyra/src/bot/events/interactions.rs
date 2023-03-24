use crate::bot::commands::{declare::handle_commands, models::Context};

pub async fn handle_app(ctx: Context) -> anyhow::Result<()> {
    handle_commands(ctx).await?;

    Ok(())
}
