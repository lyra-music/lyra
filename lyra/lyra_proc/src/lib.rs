extern crate proc_macro;

mod declare_kinds;
mod models;

use models::Args;
use proc_macro::TokenStream;

use declare_kinds::impl_declare_kinds;

#[proc_macro_attribute]
pub fn declare_kinds(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as Args);
    let input = syn::parse_macro_input!(input as syn::ItemStruct);

    impl_declare_kinds(&args, &input)
}
