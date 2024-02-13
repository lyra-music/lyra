extern crate proc_macro;

mod command;
mod config_access;
mod hook;
mod model;

use proc_macro::TokenStream;

#[proc_macro_derive(BotCommandGroup)]
pub fn bot_command_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    command::impl_lyra_command_group(&input)
}

#[proc_macro]
pub fn view_access_ids(attr: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as model::Args);

    config_access::impl_view_access_ids(&args)
}

#[proc_macro_attribute]
// Attributions: https://github.com/arqunis/hook
pub fn hook(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let fun = syn::parse_macro_input!(input as syn::ItemFn);

    hook::impl_hook(fun)
}
