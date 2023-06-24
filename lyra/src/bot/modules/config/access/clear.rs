use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio::task::JoinSet;
use twilight_gateway::Event;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    channel::message::component::{TextInput, TextInputStyle},
};

use super::AccessCategory;
use crate::bot::{
    commands::{
        models::{App, LyraCommand},
        Context,
    },
    ext::utils::FlagsPrettify,
    lib::consts::{
        exit_codes::{DUBIOUS, NOTICE},
        misc::DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT,
        texts::NO_ROWS_AFFECTED_MESSAGE,
    },
    modules::config::access::AccessCategoryFlags,
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "clear",
    desc = "Clears all currently configured access controls for channels, roles or members"
)]
pub struct Clear {
    #[command(desc = "Which category(s)?")]
    category: AccessCategory,
}

#[async_trait]
impl LyraCommand for Clear {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        let category_flags: AccessCategoryFlags = self.category.into();

        let mut set = JoinSet::new();

        category_flags.iter_names_as_column().for_each(|c| {
            let db = ctx.db().clone();
            let g = ctx.guild_id_unchecked().get() as i64;

            set.spawn(async move {
                sqlx::query(&format!(
                    "--sql
                DELETE FROM {c} WHERE guild = $1;"
                ))
                .bind(g)
                .execute(&db)
                .await
            });
        });

        // TODO: warn for destructive command
        let text_input = TextInput {
            custom_id: "destructive-command-confirmation-text-input".into(),
            label: "Are you sure?".into(),
            max_length: None,
            min_length: None,
            required: true.into(),
            placeholder: Some(r#"Type "YES" (All Caps) to confirm..."#.into()),
            style: TextInputStyle::Short,
            value: None,
        };

        ctx.respond_modal(
            "destructive-command-confirmation",
            "Running A Destructive Command",
            [text_input],
        )
        .await?;

        let author_id = ctx.author_id();
        let wait_for_component_future =
            ctx.bot()
                .standby()
                .wait_for(ctx.guild_id_unchecked(), move |e: &Event| {
                    if let Event::InteractionCreate(i) = e {
                        if matches!(i.kind, InteractionType::ModalSubmit)
                            && i.author_id() == Some(author_id)
                        {
                            return true;
                        }
                    }
                    false
                });

        let wait_for_component_future = tokio::time::timeout(
            Duration::from_secs(DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT.into()),
            wait_for_component_future,
        )
        .await;

        let modal_submit = match wait_for_component_future {
            Ok(Ok(Event::InteractionCreate(interaction))) => match interaction.data {
                Some(InteractionData::ModalSubmit(ref modal_submit))
                    if !modal_submit.components[0].components[0]
                        .value
                        .as_ref()
                        .is_some_and(|s| s == "YES") =>
                {
                    ctx.bot()
                        .respond_to(
                            &interaction,
                            format!("{} Cancled running a destructive command.", NOTICE),
                        )
                        .await?;
                    return Ok(());
                }
                Some(InteractionData::ModalSubmit(_)) => interaction,
                _ => unreachable!(),
            },
            Ok(Ok(_)) => unreachable!(),
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                ctx.followup_ephem(&format!(
                    "{} Timed out waiting for destructive command confirmation.",
                    DUBIOUS
                ))
                .await?;
                return Ok(());
            }
        };

        let mut rows_affected = 0;
        while let Some(res) = set.join_next().await {
            let res = res??;
            rows_affected += res.rows_affected();
        }

        if rows_affected == 0 {
            ctx.bot()
                .ephem_to(
                    &modal_submit,
                    format!("{} {}", DUBIOUS, NO_ROWS_AFFECTED_MESSAGE),
                )
                .await?;
            return Ok(());
        }

        ctx.bot()
            .respond_to(
                &modal_submit,
                format!(
                    "🔐🧹 Cleared all access controls for **{}**.",
                    category_flags.prettify_code()
                ),
            )
            .await?;
        Ok(())
    }
}
