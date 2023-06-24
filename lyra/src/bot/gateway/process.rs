use anyhow::Result;
use twilight_gateway::Event;
use twilight_model::application::interaction::InteractionType;

use super::{guilds, models::Process, shards, ContextedLyra};
use crate::bot::{commands, voice};

impl ContextedLyra {
    pub async fn process(self) -> Result<()> {
        match self.event {
            Event::Ready(ref e) => {
                let ctx = shards::ReadyContext::from_ready(e, &self);
                ctx.process().await
            }
            Event::GuildCreate(ref e) => {
                let ctx = guilds::Context::from_guild_events(e, &self);
                ctx.process().await
            }
            Event::GuildDelete(ref e) => {
                let ctx = guilds::Context::from_guild_events(e, &self);
                ctx.process().await
            }
            Event::InteractionCreate(ref e) => match e.kind {
                InteractionType::ApplicationCommand => {
                    let ctx = commands::Context::from_app_interaction(e.clone(), self.into());
                    ctx.process().await
                }
                _ => Ok(()),
            },
            Event::VoiceStateUpdate(ref e) => {
                let ctx = voice::Context::from_voice_state_update(e, &self);
                ctx.process().await
            }
            _ => Ok(()),
        }
    }
}
