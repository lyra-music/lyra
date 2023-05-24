use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_lavalink::model::Destroy;
use twilight_model::{
    application::interaction::application_command::InteractionChannel,
    gateway::payload::{incoming::MessageCreate, outgoing::UpdateVoiceState},
};

use crate::bot::commands::models::{Context, LyraCommand};

#[derive(CreateCommand, CommandModel)]
#[command(name = "join", desc = "Join a voice/stage channel")]
pub struct Join {
    #[command(
        desc = "Which channel? (if not given, your currently connected channel)",
        channel_types = "guild_voice guild_stage_voice"
    )]
    channel: Option<InteractionChannel>,
}

#[async_trait]
impl LyraCommand for Join {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()> {
        // let bot = ctx.bot();
        // ctx.respond("What's the channel ID you want me to join?")
        //     .await?;

        // let author_id = ctx.author().id;
        // let msg = bot
        //     .standby()
        //     .wait_for_message(ctx.channel_id(), move |new_msg: &MessageCreate| {
        //         new_msg.author.id == author_id
        //     })
        //     .await?;
        // let channel_id = msg.content.parse()?;
        // let guild_id = ctx.guild_id().expect("known to be present");

        // bot.sender().command(&UpdateVoiceState::new(
        //     guild_id,
        //     Some(channel_id),
        //     false,
        //     false,
        // ))?;

        // ctx.respond(&format!("Joined <#{channel_id}>!")).await?;
        ctx.respond(&format!("```rs\n{:#?}```", self.channel))
            .await?;

        Ok(())
    }
}

pub async fn leave(ctx: Context) -> anyhow::Result<()> {
    let bot = ctx.bot();

    tracing::debug!(
        "leave command in channel {} by {}",
        ctx.channel_id(),
        ctx.author().name
    );

    let guild_id = ctx.guild_id().unwrap();
    let player = ctx.lavalink().player(guild_id).await.unwrap();
    player.send(Destroy::from(guild_id))?;
    bot.sender()
        .command(&UpdateVoiceState::new(guild_id, None, false, false))?;

    ctx.respond("Left the channel").await?;

    Ok(())
}
