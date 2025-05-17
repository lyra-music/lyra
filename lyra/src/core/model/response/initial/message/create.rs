use twilight_model::channel::message::{AllowedMentions, MessageFlags};

use crate::core::model::response::Respond;

use super::{InteractionResponseType2, ResponseBuilder};

macro_rules! generate_hid_variants {
    ($($name: ident => $emoji: ident),+$(,)?) => {
        $(
            #[inline]
            fn $name(&mut self, content: impl ::std::convert::Into<::std::string::String>) -> ResponseBuilder<'_, Self>
            where
                Self: ::std::marker::Sized,
            {
                self.hid(format!("{} {}", $crate::core::r#const::exit_code::$emoji, content.into()))
            }
        )+
    }
}

pub trait RespondWithMessage: Respond {
    fn respond(&mut self) -> ResponseBuilder<'_, Self>
    where
        Self: Sized,
    {
        ResponseBuilder::default()
            .inner(self)
            .interaction_response_type(InteractionResponseType2::ChannelMessageWithSource)
            .allowed_mentions(AllowedMentions::default())
    }
    #[inline]
    fn out(&mut self, content: impl Into<String>) -> ResponseBuilder<'_, Self>
    where
        Self: Sized,
    {
        self.respond().content(content)
    }
    #[inline]
    fn hid(&mut self, content: impl Into<String>) -> ResponseBuilder<'_, Self>
    where
        Self: Sized,
    {
        self.respond()
            .flags(MessageFlags::EPHEMERAL)
            .content(content)
    }

    generate_hid_variants! {
        note => NOTICE,
        susp => DUBIOUS,
        warn => WARNING,
        wrng => INVALID,
        nope => PROHIBITED,
        blck => FORBIDDEN,
        erro => KNOWN_ERROR,
        unkn => UNKNOWN_ERROR
    }
}
