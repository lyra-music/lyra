use crate::bot::commands::{define::COMMANDS, models::Context};

pub async fn handle(ctx: Context) -> anyhow::Result<()> {
    if let Some((c, _)) = COMMANDS.iter().find(|(_, c)| c.name == ctx.command_name()) {
        c.callback(ctx).await?;
    }
    Ok(())
}
