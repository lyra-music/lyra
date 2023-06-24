extern crate proc_macro;

mod commands;
mod config_access;
mod declare_kinds;
mod models;

use config_access::impl_view_access_ids;
use models::Args;
use proc_macro::TokenStream;

use commands::{impl_check, impl_lyra_command_group};
use declare_kinds::impl_declare_kinds;
use syn::DeriveInput;

#[proc_macro_attribute]
pub fn declare_kinds(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);
    let input = syn::parse_macro_input!(input as syn::ItemStruct);

    impl_declare_kinds(&args, &input)
}

// TODO: Make the check macro an `ImplItemFn` attribute macro once async traits are stable.
#[proc_macro]
pub fn check(attr: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);

    impl_check(&args)
}

#[proc_macro_derive(LyraCommandGroup)]
pub fn lyra_command_group(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    impl_lyra_command_group(&input)
}

#[proc_macro]
pub fn view_access_ids(attr: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);

    impl_view_access_ids(&args)
}
