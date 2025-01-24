pub mod application {
    use tokio::sync::OnceCell;
    use twilight_model::{
        guild::Emoji,
        id::{marker::ApplicationMarker, Id},
    };

    use crate::{core::model::HttpAware, error::core::DeserialiseBodyFromHttpError};

    static ID: OnceCell<Id<ApplicationMarker>> = OnceCell::const_new();
    static EMOJIS: OnceCell<&'static [Emoji]> = OnceCell::const_new();

    pub async fn id(
        cx: &(impl HttpAware + Sync),
    ) -> Result<Id<ApplicationMarker>, DeserialiseBodyFromHttpError> {
        ID.get_or_try_init(|| async {
            let application = cx.http().current_user_application().await?.model().await?;
            Ok(application.id)
        })
        .await
        .copied()
    }

    pub async fn emojis(
        cx: &(impl HttpAware + Sync),
    ) -> Result<&'static [Emoji], DeserialiseBodyFromHttpError> {
        EMOJIS
            .get_or_try_init(|| async {
                let application_id = id(cx).await?;
                let req = cx.http().get_application_emojis(application_id);
                Ok(&*req.await?.model().await?.items.leak())
            })
            .await
            .copied()
    }
}

pub mod component {
    use std::sync::LazyLock;

    use rand::{distributions::Alphanumeric, Rng};

    pub struct NowPlayingButtonIds {
        pub shuffle: &'static str,
        pub previous: &'static str,
        pub play_pause: &'static str,
        pub next: &'static str,
        pub repeat: &'static str,
    }

    pub enum NowPlayingButtonType {
        Shuffle,
        Previous,
        PlayPause,
        Next,
        Repeat,
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
            let mut button_id_iter = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .map(char::from);

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
