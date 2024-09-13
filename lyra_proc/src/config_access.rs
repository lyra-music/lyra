use heck::ToTitleCase;
use itertools::Itertools;
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::model::Args;

pub fn impl_view_access_ids(Args(categories): &Args) -> TokenStream {
    let column_names = categories
        .iter()
        .map(|c| match c.to_string().as_str() {
            "users" => "usr",
            "roles" => "rol",
            "threads" => "xch",
            "text_channels" => "tch",
            "voice_channels" => "vch",
            "category_channels" => "cch",
            c => panic!("invalid access category: {c}"),
        })
        .map(|c| format!("{c}_access"));

    let access_queries = column_names
        .clone()
        .map(|t| format!("SELECT id FROM {t} WHERE guild = $1;",));

    let mode_queries = format!(
        "SELECT {} FROM guild_configs WHERE id = $1",
        column_names.clone().join(", ")
    );

    let column_names_ident = column_names
        .map(|c| Ident::new(&c, Span::call_site().into()))
        .collect::<Vec<_>>();

    let id_markers = categories
        .iter()
        .map(|c| match c.to_string().as_ref() {
            "users" => "UserMarker",
            "roles" => "RoleMarker",
            "threads" | "text_channels" | "voice_channels" | "category_channels" => "ChannelMarker",
            c => panic!("invalid access category: {c}"),
        })
        .map(|m| Ident::new(m, Span::call_site().into()));

    let category_names = categories.iter().map(|c| c.to_string().to_title_case());

    quote! {
        use ::itertools::Itertools;
        use ::twilight_mention::Mention;
        use ::twilight_model::id::{
            marker::{ChannelMarker, RoleMarker, UserMarker},
            Id,
        };
        use ::lyra_ext::logical_bind::LogicalBind;
        use ::twilight_util::builder::embed::EmbedFieldBuilder;

        use crate::component::config::access::mode::AccessModeDisplay;
        use crate::gateway::GuildIdAware;
        use crate::core::r#const::text::EMPTY_EMBED_FIELD;

        struct __AccessView {
            id: i64,
        }

        struct __AccessModeView {
            #(
                #column_names_ident: Option<bool>,
            )*
        }

        let guild_id = ctx.guild_id().get() as i64;
        let db = ctx.db();

        let __access_modes = sqlx::query_as!(
            __AccessModeView,
            #mode_queries,
            guild_id,
        )
        .fetch_one(db).await?;

        #(
            let #categories = async move {
                sqlx::query_as!(
                    __AccessView,
                    #access_queries,
                    guild_id,
                )
                .fetch_all(db).await
            };

        )*
        let (#(#categories,)*) = tokio::try_join!(#(#categories,)*)?;

        #(
            let #categories = #categories.iter().map(|v| {Id::<#id_markers>::new(v.id as u64).mention()}).join(" ");
            let embed = embed.field(
                EmbedFieldBuilder::new(
                    format!(
                        "{} {}", 
                        __access_modes.#column_names_ident.display_icon(),
                        #category_names
                    ), #categories.or(EMPTY_EMBED_FIELD)
                ).inline()
            );
        )*
    }
    .into()
}
