use std::env;

use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

pub fn impl_read_play_sources_as(ty: &Ident) -> TokenStream {
    dotenvy::dotenv().ok();

    let (yt, ytm, sc) = ("Youtube", "Youtube Music", "SoundCloud");
    let mut sources = vec![yt, ytm, sc];
    let deezer =
        if env::var("PLUGINS_LAVASRC_SOURCES_DEEZER").is_ok_and(|x| x.parse().is_ok_and(|y| y)) {
            let dz = "Deezer";
            sources.push(dz);
            let (dzsearch, dzisrc) = (format!("{dz} (Search Query)"), format!("{dz} (ISRC)"));
            quote! {
                #[option(name = #dzsearch, value = "dzsearch:")]
                DeezerQuery,
                #[option(name = #dzisrc, value = "dzisrc:")]
                DeezerIsrc,
            }
        } else {
            quote!()
        };
    let spotify =
        if env::var("PLUGINS_LAVASRC_SOURCES_SPOTIFY").is_ok_and(|x| x.parse().is_ok_and(|y| y)) {
            let sp = "Spotify";
            sources.push(sp);
            quote! {
                #[option(name = #sp, value = "spsearch:")]
                Spotify,
            }
        } else {
            quote!()
        };

    let (sources_len, sources_arr) = (sources.len(), quote!([#(#sources,)*]));
    quote! {
        #[derive(
            ::twilight_interactions::command::CommandOption,
            ::twilight_interactions::command::CreateOption,
            ::std::default::Default
        )]
        pub enum #ty {
            #[default]
            #[option(name = #yt, value = "ytsearch:")]
            Youtube,
            #[option(name = #ytm, value = "ytmsearch:")]
            YoutubeMusic,
            #[option(name = #sc, value = "scsearch:")]
            SoundCloud,
            #deezer
            #spotify
        }

        impl #ty {
            pub const fn values() -> [&'static str; #sources_len] {
                #sources_arr
            }
        }
    }
    .into()
}
