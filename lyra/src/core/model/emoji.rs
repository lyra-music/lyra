use std::sync::OnceLock;

use twilight_model::channel::message::EmojiReactionType;

use crate::error::core::DeserializeBodyFromHttpError;

use super::BotState;

macro_rules! generate_emojis {
    ($ (($name: ident, $default: expr)) ,* $(,)? ) => {$(
        pub async fn $name(
            bot: &BotState,
        ) -> Result<&'static EmojiReactionType, DeserializeBodyFromHttpError> {
            ::paste::paste! {
                static [<$name:upper>]: OnceLock<EmojiReactionType> = OnceLock::new();
                if let Some(emoji) = [<$name:upper>].get() {
                    return Ok(emoji);
                }
            }

            let emojis = bot.application_emojis().await?;
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
    (shuffle_off, "🔀"),
    (shuffle_on, "🔀"),
    (previous, "⏮️"),
    (play, "▶️"),
    (pause, "⏸️"),
    (next, "⏭️"),
    (repeat_off, "➡️"),
    (repeat_all, "🔁"),
    (repeat_track, "🔂"),
];
