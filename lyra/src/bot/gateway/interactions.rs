use crate::bot::commands::{
    declare::handle_commands,
    models::{App, Context},
};

pub async fn handle_app(ctx: Context<App>) -> anyhow::Result<()> {
    handle_commands(ctx).await?;

    Ok(())
}
