extern crate proc_macro;

mod commands;
mod declare_kinds;
mod models;

use models::Args;
use proc_macro::TokenStream;

use commands::{impl_check, impl_err, impl_out};
use declare_kinds::impl_declare_kinds;

#[proc_macro_attribute]
pub fn declare_kinds(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);
    let input = syn::parse_macro_input!(input as syn::ItemStruct);

    impl_declare_kinds(&args, &input)
}

#[proc_macro]
pub fn out(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::Expr);

    impl_out(&input)
}

#[proc_macro]
pub fn err(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::Expr);

    impl_err(&input)
}

#[proc_macro]
pub fn check(attr: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);

    impl_check(&args)
}
