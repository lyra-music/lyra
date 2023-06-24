use anyhow::Result;
use twilight_model::{
    channel::ChannelType,
    guild::Permissions,
    id::{marker::ChannelMarker, Id},
};

use super::{
    errors::{AlreadyInVoiceError, ConnectionError, Error},
    models::App,
    Context,
};
use crate::bot::{lib::models::Cacheful, modules::config::access::AccessCalculatorBuilder};

pub static DJ_PERMISSIONS: Permissions = Permissions::MOVE_MEMBERS
    .union(Permissions::MUTE_MEMBERS)
    .union(Permissions::DEAFEN_MEMBERS);
pub static ACCESS_MANAGER_PERMISSIONS: Permissions =
    Permissions::MANAGE_ROLES.union(Permissions::MANAGE_CHANNELS);

pub async fn user_allowed_in(ctx: &Context<App>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {return Ok(());};
    let author_permissions = ctx.author_permissions();

    if author_permissions.contains(ACCESS_MANAGER_PERMISSIONS)
        || author_permissions.contains(Permissions::ADMINISTRATOR)
    {
        return Ok(());
    }

    let channel = ctx.channel();
    let mut access_calculator_builder = AccessCalculatorBuilder::new(guild_id, ctx.db().clone())
        .user(ctx.author_id())
        .role(ctx.member().roles.iter());
    match channel.kind {
        ChannelType::PublicThread
        | ChannelType::PrivateThread
        | ChannelType::AnnouncementThread => {
            let parent_id = channel
                .parent_id
                .expect("threads must have a parent channel");
            access_calculator_builder = access_calculator_builder
                .thread(channel.id)
                .text_channel(parent_id);
        }
        ChannelType::GuildVoice | ChannelType::GuildStageVoice => {
            let channel_id = channel.id;
            access_calculator_builder = access_calculator_builder
                .text_channel(channel_id)
                .voice_channel(channel_id)
        }
        _ => {
            access_calculator_builder = access_calculator_builder.text_channel(channel.id);
            if let Some(category_channel_id) = channel.parent_id {
                access_calculator_builder =
                    access_calculator_builder.category_channel(category_channel_id)
            }
        }
    };

    let user_allowed_to_use_commands = access_calculator_builder.build().await?.calculate();
    if !user_allowed_to_use_commands {
        return Err(Error::UserNotAllowed.into());
    }
    Ok(())
}

pub async fn user_allowed_to_use(
    ctx: &Context<App>,
    channel_id: Id<ChannelMarker>,
    channel_parent_id: Option<Id<ChannelMarker>>,
) -> Result<()> {
    let guild_id = ctx.guild_id_unchecked();
    let author_permissions = ctx.author_permissions();
    if author_permissions.contains(ACCESS_MANAGER_PERMISSIONS)
        || author_permissions.contains(Permissions::ADMINISTRATOR)
    {
        return Ok(());
    }

    let mut access_calculator_builder =
        AccessCalculatorBuilder::new(guild_id, ctx.db().clone()).voice_channel(channel_id);

    if let Some(parent_id) = channel_parent_id {
        access_calculator_builder = access_calculator_builder.category_channel(parent_id)
    }

    let allowed_to_use_channel = access_calculator_builder.build().await?.calculate();

    if !allowed_to_use_channel {
        return Err(Error::UserNotAllowed.into());
    };
    Ok(())
}

pub fn noone_else_in_voice(ctx: &Context<App>, channel_id: Id<ChannelMarker>) -> Result<(), Error> {
    let author_permissions = ctx.author_permissions();
    if author_permissions.contains(DJ_PERMISSIONS)
        || author_permissions.contains(Permissions::ADMINISTRATOR)
    {
        return Ok(());
    }

    let someone_else_in_voice = ctx
        .cache()
        .voice_channel_states(channel_id)
        .map(|mut states| {
            states.any(|v| {
                !ctx.cache()
                    .user(v.user_id())
                    .expect("user of `v.user_id()` must exist in the cache")
                    .bot
                    && v.user_id() != ctx.author_id()
            })
        })
        .ok_or(Error::Cache)?;

    if someone_else_in_voice {
        return Err(Error::Connection {
            channel_id,
            source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SomeoneElseInVoice),
        });
    }

    Ok(())
}
