use anyhow::Result;

use super::models::ContextedLyra;

impl ContextedLyra {
    pub async fn process(self) -> Result<()> {
        // TODO: Handle lavalink events
        // match ctx.event {
        //     _ => {}
        // }

        Ok(())
    }
}
