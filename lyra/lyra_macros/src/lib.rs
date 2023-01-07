extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_attribute]
pub fn cached_oxidized(_: TokenStream, input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    impl_cached_oxidized(&ast)
}

fn impl_cached_oxidized(ast: &syn::ItemFn) -> TokenStream {
    let sig = &ast.sig;
    let block = &ast.block;

    let ident = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;

    let gen = quote! {
        #[cached]
        fn #ident(#inputs) #output {
            #block.map_err(|err| LyraErr::Image(err.into()))
        }
    };
    gen.into()
}
