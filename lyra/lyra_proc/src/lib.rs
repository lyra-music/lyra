extern crate proc_macro;

mod command;
mod config_access;
mod model;

use config_access::impl_view_access_ids;
use model::Args;
use proc_macro::TokenStream;

use command::impl_lyra_command_group;
use syn::DeriveInput;


#[proc_macro_derive(BotCommandGroup)]
pub fn bot_command_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    impl_lyra_command_group(&input)
}

#[proc_macro]
pub fn view_access_ids(attr: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);

    impl_view_access_ids(&args)
}
