use twilight_model::{
    guild::Permissions,
    id::{marker::ChannelMarker, Id},
};

use super::{
    errors::{AlreadyInVoiceError, ConnectionError, Error},
    models::Context,
};

pub static DJ_PERMISSIONS: Permissions =
    Permissions::MOVE_MEMBERS.union(Permissions::MUTE_MEMBERS.union(Permissions::DEAFEN_MEMBERS));

pub fn noone_else_in_voice(ctx: &Context, channel_id: Id<ChannelMarker>) -> Result<(), Error> {
    let author_permissions = ctx.author_permissions();

    if author_permissions.contains(DJ_PERMISSIONS | Permissions::ADMINISTRATOR) {
        return Ok(());
    }

    let someone_else_in_voice = match ctx.cache().voice_channel_states(channel_id) {
        Some(mut states) => states.any(|v| {
            !ctx.cache()
                .user(v.user_id())
                .expect("user must exists in the cache")
                .bot
                && v.user_id() != ctx.author_id()
        }),
        None => return Err(Error::Cache),
    };

    if someone_else_in_voice {
        return Err(Error::Connection {
            channel_id,
            source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SomeoneElseInVoice),
        });
    }

    Ok(())
}
