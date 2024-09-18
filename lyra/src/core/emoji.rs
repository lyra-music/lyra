use std::sync::OnceLock;

use twilight_model::channel::message::EmojiReactionType;

use crate::{core::model::HttpAware, error::core::DeserialiseBodyFromHttpError};

macro_rules! generate_emojis {
    ($ (($name: ident, $default: expr)) ,* $(,)? ) => {$(
        pub async fn $name(_cx: &(impl HttpAware + Sync)) -> Result<&'static EmojiReactionType, DeserialiseBodyFromHttpError> {
            ::paste::paste! {
                static [<$name:upper>]: OnceLock<EmojiReactionType> = OnceLock::new();
                if let Some(emoji) = [<$name:upper>].get() {
                    return Ok(emoji);
                }
            }

            let emojis: &'static [twilight_model::guild::Emoji] = &[];

            // FIXME: https://github.com/twilight-rs/twilight/issues/2373
            // let emojis = crate::core::r#static::application::emojis(cx).await?;
            let emoji = emojis.iter().find(|e| e.name == stringify!($name));
            let reaction = emoji.map_or(
                {
                    EmojiReactionType::Unicode {
                        name: String::from($default),
                    }
                },
                |emoji| EmojiReactionType::Custom {
                    animated: emoji.animated,
                    id: emoji.id,
                    name: Some(emoji.name.clone()),
                },
            );
            ::paste::paste!(Ok([<$name:upper>].get_or_init(|| reaction)))
        }
    )*};
}

generate_emojis![
    (shuffle_off, "⬅️"),
    (shuffle_on, "🔀"),
    (previous, "⏮️"),
    (play, "▶️"),
    (pause, "⏸️"),
    (next, "⏭️"),
    (repeat_off, "➡️"),
    (repeat_all, "🔁"),
    (repeat_track, "🔂"),
];
