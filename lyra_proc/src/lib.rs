extern crate proc_macro;

mod command;
mod config_access;
mod equaliser_preset;
mod model;
mod play_source;

use proc_macro::TokenStream;

#[proc_macro_derive(BotCommandGroup)]
pub fn bot_command_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    command::impl_bot_command_group(&input, false)
}

#[proc_macro_derive(BotGuildCommandGroup)]
pub fn bot_guild_command_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    command::impl_bot_command_group(&input, true)
}

#[proc_macro_derive(BotAutocompleteGroup)]
pub fn bot_autocomplete_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    command::impl_bot_autocomplete_group(&input, false)
}

#[proc_macro_derive(BotGuildAutocompleteGroup)]
pub fn bot_guild_autocomplete_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    command::impl_bot_autocomplete_group(&input, true)
}

#[proc_macro]
pub fn view_access_ids(attr: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as model::Args);

    config_access::impl_view_access_ids(&args)
}

#[proc_macro]
pub fn read_equaliser_presets_as(ty: TokenStream) -> TokenStream {
    let ty = syn::parse_macro_input!(ty as syn::Ident);

    equaliser_preset::impl_read_equaliser_presets_as(&ty)
}

#[proc_macro]
pub fn read_play_sources_as(ty: TokenStream) -> TokenStream {
    let ty = syn::parse_macro_input!(ty as syn::Ident);

    play_source::impl_read_play_sources_as(&ty)
}
