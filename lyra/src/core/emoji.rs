macro_rules! generate_emojis {
    ($ (($name: ident, $default: expr)) ,* $(,)? ) => {$(
        pub async fn $name(cx: &(impl $crate::core::model::HttpAware + ::std::marker::Sync))
            -> ::std::result::Result<
                &'static ::twilight_model::channel::message::EmojiReactionType,
                $crate::error::core::DeserialiseBodyFromHttpError> {
            ::paste::paste! {
                static [<$name:upper>]: ::std::sync::OnceLock<::twilight_model::channel::message::EmojiReactionType> =
                    ::std::sync::OnceLock::new();
                if let ::std::option::Option::Some(emoji) = [<$name:upper>].get() {
                    return ::std::result::Result::Ok(emoji);
                }
            }

            let emojis = $crate::core::statik::application::emojis(cx).await?;
            let emoji = emojis.iter().find(|e| e.name == ::std::stringify!($name));
            let reaction = emoji.map_or(
                {
                    ::twilight_model::channel::message::EmojiReactionType::Unicode {
                        name: ::std::string::String::from($default),
                    }
                },
                |emoji| ::twilight_model::channel::message::EmojiReactionType::Custom {
                    animated: emoji.animated,
                    id: emoji.id,
                    name: ::std::option::Option::Some(emoji.name.clone()),
                },
            );
            ::paste::paste!(::std::result::Result::Ok([<$name:upper>].get_or_init(|| reaction)))
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
