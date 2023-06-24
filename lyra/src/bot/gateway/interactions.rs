use anyhow::Result;
use async_trait::async_trait;
use twilight_mention::Mention;

use crate::bot::{
    commands::{
        checks::DJ_PERMISSIONS,
        errors::{AlreadyInVoiceError, ConnectionError, Error},
        macros::{crit, err, hid, nope},
        models::{App, Context},
    },
    ext::utils::BitFlagsPrettify,
};

use super::models::Process;

#[async_trait]
impl Process for Context<App> {
    async fn process(self) -> Result<()> {
        if let Err(e) = self.clone().execute().await {
            match e.downcast() {
                Ok(Error::Cache) => {
                    crit!(
                        "Something isn't working at the moment, try again later.",
                        self
                    );
                }
                Ok(Error::UserNotAllowed) => {
                    nope!("You are not allowed to use commands in this context.", self);
                }
                Ok(Error::Connection {
                    channel_id,
                    source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SomeoneElseInVoice),
                }) => {
                    nope!(format!(
                        "Someone else is using the bot in {}; You'll need **{}** permissions to do that.", 
                        channel_id.mention(),
                        DJ_PERMISSIONS.prettify()
                    ), self);
                }
                Ok(other) => {
                    err!(
                        format!("Something went wrong: ```rs\n{other:#?}```"),
                        self,
                        !
                    );
                    return Err(other.into());
                }
                Err(other) => {
                    crit!(format!(
                    "Something unexpectedly went wrong: ```rs\n{other:#?}``` Please report this to the bot developers."
                ), self, !);
                    return Err(other);
                }
            }
        };

        Ok(())
    }
}
