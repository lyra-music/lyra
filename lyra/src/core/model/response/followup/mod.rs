use create::FollowupBuilder;
use twilight_model::{
    channel::message::MessageFlags,
    id::{Id, marker::MessageMarker},
};
use update::UpdateFollowupBuilder;

use super::{EmptyResponseResult, Respond};

pub mod create;
pub mod update;

macro_rules! generate_hid_variants {
    ($($name: ident => $emoji: ident),+$(,)?) => {
        $(
            ::paste::paste! {
                #[inline]
                #[allow(unused)]
                fn [<$name f>](&self, content: impl ::std::convert::Into<::std::string::String>) -> FollowupBuilder<'_, Self>
                where
                    Self: ::std::marker::Sized,
                {
                    self.hidf(format!("{} {}", $crate::core::konst::exit_code::$emoji, content.into()))
                }
            }
        )+
    }
}

pub trait Followup: Respond {
    fn raw_followup(&self) -> FollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        FollowupBuilder::new(self)
    }
    #[inline]
    #[expect(unused)]
    fn outf(&self, content: impl Into<String>) -> FollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        self.raw_followup().content(content.into())
    }
    #[inline]
    fn hidf(&self, content: impl Into<String>) -> FollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        self.raw_followup()
            .flags(MessageFlags::EPHEMERAL)
            .content(content.into())
    }

    #[inline]
    fn update_followup(
        &self,
        message_id: impl Into<Id<MessageMarker>>,
    ) -> UpdateFollowupBuilder<'_, Self>
    where
        Self: Sized,
    {
        UpdateFollowupBuilder::new(self, message_id.into())
    }

    #[inline]
    #[expect(unused)]
    async fn delete_followup(
        &self,
        message_id: impl Into<Id<MessageMarker>>,
    ) -> EmptyResponseResult {
        self.interaction_client()
            .delete_followup(self.interaction_token(), message_id.into())
            .await
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
