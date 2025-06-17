pub mod application {
    use std::sync::OnceLock;

    use tokio::sync::OnceCell;
    use twilight_model::{
        guild::Emoji,
        id::{Id, marker::ApplicationMarker},
    };

    use crate::{core::model::HttpAware, error::core::DeserialiseBodyFromHttpError};

    static ID: OnceLock<Id<ApplicationMarker>> = OnceLock::new();
    static EMOJIS: OnceCell<&'static [Emoji]> = OnceCell::const_new();

    pub fn set_id(id: Id<ApplicationMarker>) {
        ID.set(id).ok();
    }

    pub fn id() -> Id<ApplicationMarker> {
        *ID.get()
            .expect("ready event should have populated the application id")
    }

    pub async fn emojis(
        cx: &(impl HttpAware + Sync),
    ) -> Result<&'static [Emoji], DeserialiseBodyFromHttpError> {
        EMOJIS
            .get_or_try_init(|| async {
                let application_id = id();
                let req = cx.http().get_application_emojis(application_id);
                Ok(&*req.await?.model().await?.items.leak())
            })
            .await
            .copied()
    }
}

pub mod component {
    use std::{fmt::Display, sync::LazyLock};

    use rand::{Rng, distr::Alphanumeric};

    pub struct NowPlayingButtonIds {
        pub shuffle: &'static str,
        pub previous: &'static str,
        pub play_pause: &'static str,
        pub next: &'static str,
        pub repeat: &'static str,
    }

    #[derive(Clone, Copy, Debug)]
    pub enum NowPlayingButtonType {
        Shuffle,
        Previous,
        PlayPause,
        Next,
        Repeat,
    }

    impl Display for NowPlayingButtonType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                Self::Shuffle => "shuffle",
                Self::Previous => "previous",
                Self::PlayPause => "play_pause",
                Self::Next => "next",
                Self::Repeat => "repeat",
            };
            f.write_str(s)
        }
    }

    impl TryFrom<&str> for NowPlayingButtonType {
        type Error = ();

        fn try_from(id: &str) -> Result<Self, Self::Error> {
            if id == NOW_PLAYING_BUTTON_IDS.next {
                Ok(Self::Next)
            } else if id == NOW_PLAYING_BUTTON_IDS.play_pause {
                Ok(Self::PlayPause)
            } else if id == NOW_PLAYING_BUTTON_IDS.previous {
                Ok(Self::Previous)
            } else if id == NOW_PLAYING_BUTTON_IDS.repeat {
                Ok(Self::Repeat)
            } else if id == NOW_PLAYING_BUTTON_IDS.shuffle {
                Ok(Self::Shuffle)
            } else {
                Err(())
            }
        }
    }

    impl NowPlayingButtonIds {
        const BUTTON_ID_LEN: usize = 100;
        fn new() -> Self {
            let mut button_id_iter = rand::rng().sample_iter(&Alphanumeric).map(char::from);

            let mut button_id_gen = || {
                button_id_iter
                    .by_ref()
                    .take(Self::BUTTON_ID_LEN)
                    .collect::<String>()
                    .leak()
            };

            Self {
                shuffle: button_id_gen(),
                previous: button_id_gen(),
                play_pause: button_id_gen(),
                next: button_id_gen(),
                repeat: button_id_gen(),
            }
        }
    }

    pub static NOW_PLAYING_BUTTON_IDS: LazyLock<NowPlayingButtonIds> =
        LazyLock::new(NowPlayingButtonIds::new);
}
