use anyhow::Result;
use async_trait::async_trait;

use super::Context;
use crate::bot::{gateway::Process, modules::connections};

#[async_trait]
impl Process for Context<'_> {
    async fn process(self) -> Result<()> {
        connections::handle_voice_state_update(&self).await?;

        Ok(())
    }
}
