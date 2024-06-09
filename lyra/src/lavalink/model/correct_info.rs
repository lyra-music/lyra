use std::borrow::Cow;

use crate::core::r#const::{lavaplayer as const_lavaplayer, text as const_text};

mod private {
    pub trait CorrectInfo {}
    impl CorrectInfo for lavalink_rs::model::track::TrackInfo {}
    impl CorrectInfo for lavalink_rs::model::track::PlaylistInfo {}
}

pub trait CorrectTrackInfo: private::CorrectInfo {
    fn incorrect_title() -> String;
    fn default_title() -> String;
    fn title(&self) -> &String;
    fn title_mut(&mut self) -> &mut String;
    fn check_title(title: &String) -> Option<&str> {
        (*title != Self::incorrect_title()).then_some(title)
    }
    fn checked_title(&self) -> Option<&str> {
        Self::check_title(self.title())
    }
    fn corrected_title(&self) -> Cow<str> {
        self.checked_title()
            .map_or_else(|| Cow::Owned(Self::default_title()), Cow::Borrowed)
    }
    fn take_and_correct_title(&mut self) -> String {
        let title = std::mem::take(self.title_mut());
        Self::check_title(&title)
            .is_some()
            .then_some(title)
            .unwrap_or_else(Self::default_title)
    }

    fn author(&self) -> &String;
    fn author_mut(&mut self) -> &mut String;

    fn incorrect_author() -> String;
    fn default_author() -> String;
    fn check_author(author: &String) -> Option<&str> {
        (*author != Self::incorrect_author()).then_some(author)
    }
    fn checked_author(&self) -> Option<&str> {
        Self::check_author(self.author())
    }
    fn corrected_author(&self) -> Cow<str> {
        self.checked_author()
            .map_or_else(|| Cow::Owned(Self::default_author()), Cow::Borrowed)
    }
    fn take_and_correct_author(&mut self) -> String {
        let author = std::mem::take(self.author_mut());
        Self::check_author(&author)
            .is_some()
            .then_some(author)
            .unwrap_or_else(Self::default_author)
    }
}

impl CorrectTrackInfo for lavalink_rs::model::track::TrackInfo {
    fn incorrect_title() -> String {
        const_lavaplayer::UNKNOWN_TITLE.to_owned()
    }
    fn default_title() -> String {
        const_text::UNTITLED_TRACK.to_owned()
    }

    fn title(&self) -> &String {
        &self.title
    }

    fn title_mut(&mut self) -> &mut String {
        &mut self.title
    }

    fn incorrect_author() -> String {
        const_lavaplayer::UNKNOWN_ARTIST.to_owned()
    }

    fn default_author() -> String {
        const_text::UNKNOWN_ARTIST.to_owned()
    }

    fn author(&self) -> &String {
        &self.author
    }

    fn author_mut(&mut self) -> &mut String {
        &mut self.author
    }
}

pub trait CorrectPlaylistInfo: private::CorrectInfo {
    fn incorrect_name() -> String;
    fn default_name() -> String;
    fn name(&self) -> &String;
    fn name_mut(&mut self) -> &mut String;
    fn check_name(title: &String) -> Option<&str> {
        (*title != Self::incorrect_name()).then_some(title)
    }
    fn checked_name(&self) -> Option<&str> {
        Self::check_name(self.name())
    }
    fn corrected_name(&self) -> Cow<str> {
        self.checked_name()
            .map_or_else(|| Cow::Owned(Self::default_name()), Cow::Borrowed)
    }
    fn take_and_correct_name(&mut self) -> String {
        let title = std::mem::take(self.name_mut());
        Self::check_name(&title)
            .is_some()
            .then_some(title)
            .unwrap_or_else(Self::default_name)
    }
}

impl CorrectPlaylistInfo for lavalink_rs::model::track::PlaylistInfo {
    fn incorrect_name() -> String {
        const_lavaplayer::UNKNOWN_TITLE.to_owned()
    }

    fn default_name() -> String {
        const_text::UNNAMED_PLAYLIST.to_owned()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}
