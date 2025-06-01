use std::env;

use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

pub fn impl_read_play_sources_as(ty: &Ident) -> TokenStream {
    dotenvy::dotenv().ok();
    let deezer =
        if env::var("PLUGINS_LAVASRC_SOURCES_DEEZER").is_ok_and(|x| x.parse().is_ok_and(|y| y)) {
            quote! {
                #[option(name = "Deezer (Search Query)", value = "dzsearch:")]
                DeezerQuery,
                #[option(name = "Deezer (ISRC)", value = "dzisrc:")]
                DeezerIsrc,
            }
        } else {
            quote!()
        };
    let spotify =
        if env::var("PLUGINS_LAVASRC_SOURCES_SPOTIFY").is_ok_and(|x| x.parse().is_ok_and(|y| y)) {
            quote! {
                #[option(name = "Spotify", value = "spsearch:")]
                Spotify,
            }
        } else {
            quote!()
        };
    quote! {
        #[derive(
            ::twilight_interactions::command::CommandOption,
            ::twilight_interactions::command::CreateOption,
            ::std::default::Default
        )]
        enum #ty {
            #[default]
            #[option(name = "Youtube", value = "ytsearch:")]
            Youtube,
            #[option(name = "Youtube Music", value = "ytmsearch:")]
            YoutubeMusic,
            #[option(name = "SoundCloud", value = "scsearch:")]
            SoundCloud,
            #deezer
            #spotify
        }
    }
    .into()
}
