use std::borrow::Cow;

use crate::core::konst::{lavaplayer as const_lavaplayer, text as const_text};

mod private {
    pub trait CorrectInfo {}
    impl CorrectInfo for lavalink_rs::model::track::TrackInfo {}
    impl CorrectInfo for lavalink_rs::model::track::PlaylistInfo {}
}

pub trait CorrectTrackInfo: private::CorrectInfo {
    const INCORRECT_TITLE: &str;
    const DEFAULT_TITLE: &str;
    fn title(&self) -> &str;
    fn take_title(&mut self) -> String;

    fn check_title(title: &str) -> Option<&str> {
        (title != Self::INCORRECT_TITLE).then_some(title)
    }
    fn checked_title(&self) -> Option<&str> {
        Self::check_title(self.title())
    }
    fn corrected_title(&self) -> &str {
        self.checked_title().unwrap_or(Self::DEFAULT_TITLE)
    }
    fn take_and_correct_title(&mut self) -> Cow<'static, str> {
        let title = self.take_title();
        if Self::check_title(&title).is_some() {
            Cow::Owned(title)
        } else {
            Cow::Borrowed(Self::DEFAULT_TITLE)
        }
    }

    const INCORRECT_AUTHOR: &'static str;
    const DEFAULT_AUTHOR: &str;
    fn author(&self) -> &str;
    fn take_author(&mut self) -> String;

    fn check_author(author: &str) -> Option<&str> {
        (author != Self::INCORRECT_AUTHOR).then_some(author)
    }
    fn checked_author(&self) -> Option<&str> {
        Self::check_author(self.author())
    }
    fn corrected_author(&self) -> &str {
        self.checked_author().unwrap_or(Self::DEFAULT_AUTHOR)
    }
    fn take_and_correct_author(&mut self) -> Cow<'static, str> {
        let author = self.take_author();
        if Self::check_author(&author).is_some() {
            Cow::Owned(author)
        } else {
            Cow::Borrowed(Self::DEFAULT_AUTHOR)
        }
    }
}

impl CorrectTrackInfo for lavalink_rs::model::track::TrackInfo {
    const INCORRECT_TITLE: &str = const_lavaplayer::UNKNOWN_TITLE;
    const DEFAULT_TITLE: &str = const_text::UNTITLED_TRACK;

    fn title(&self) -> &str {
        &self.title
    }

    fn take_title(&mut self) -> String {
        std::mem::take(&mut self.title)
    }

    const INCORRECT_AUTHOR: &str = const_lavaplayer::UNKNOWN_ARTIST;
    const DEFAULT_AUTHOR: &str = const_text::UNKNOWN_ARTIST;

    fn author(&self) -> &str {
        &self.author
    }

    fn take_author(&mut self) -> String {
        std::mem::take(&mut self.author)
    }
}

pub trait CorrectPlaylistInfo: private::CorrectInfo {
    const INCORRECT_NAME: &str;
    const DEFAULT_NAME: &str;
    fn name(&self) -> &str;
    fn take_name(&mut self) -> String;
    fn check_name(title: &str) -> Option<&str> {
        (title != Self::INCORRECT_NAME).then_some(title)
    }
    fn checked_name(&self) -> Option<&str> {
        Self::check_name(self.name())
    }
    fn corrected_name(&self) -> &str {
        self.checked_name().unwrap_or(Self::DEFAULT_NAME)
    }
    fn take_and_correct_name(&mut self) -> Cow<'static, str> {
        let name = self.take_name();
        if Self::check_name(&name).is_some() {
            Cow::Owned(name)
        } else {
            Cow::Borrowed(Self::DEFAULT_NAME)
        }
    }
}

impl CorrectPlaylistInfo for lavalink_rs::model::track::PlaylistInfo {
    const INCORRECT_NAME: &str = const_lavaplayer::UNKNOWN_TITLE;
    const DEFAULT_NAME: &str = const_text::UNNAMED_PLAYLIST;
    fn name(&self) -> &str {
        &self.name
    }
    fn take_name(&mut self) -> String {
        std::mem::take(&mut self.name)
    }
}
