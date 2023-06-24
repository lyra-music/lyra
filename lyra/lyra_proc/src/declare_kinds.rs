use std::ops::Deref;

use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{self, Ident};

use crate::models::Args;

pub(super) fn impl_declare_kinds(kinds: &Args, input: &syn::ItemStruct) -> TokenStream {
    let ref kinds = kinds.deref();
    let _kinds = kinds.iter().map(|i| i.to_string()).collect::<Vec<_>>();
    let mut _kind_snake = _kinds.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>();
    let inter_types = _kinds.iter().map(|k| {
        let k_str = match k.as_ref() {
            "App" => "ApplicationCommand",
            "Component" => "MessageComponent",
            "Modal" => "ModalSubmit",
            "Autocomplete" => "ApplicationCommandAutocomplete",
            _ => panic!("unknown kind: {}", &k),
        };
        Ident::new(&k_str, Span::call_site().into()).to_owned()
    });
    let fn_idents = _kind_snake.iter_mut().map(|k| {
        Ident::new(
            &format!("from_{}_interaction", &k),
            Span::call_site().into(),
        )
    });
    let panic_msgs = _kinds
        .iter()
        .map(|kc| format!("`interaction.kind` must be `{}`", &kc));

    quote! {
        #input

        #(
        #[derive(Copy, Clone)]
        pub struct #kinds;

        impl ContextKind for #kinds {}

        impl Context<#kinds> {
            pub fn #fn_idents(event: Box<InteractionCreate>, bot: Arc<ContextedLyra>) -> Context<#kinds> {
                if let InteractionType::#inter_types = event.kind {
                    return Self {
                        bot,
                        inner: event,
                        kind: PhantomData::<#kinds>,
                    };
                }
                panic!(#panic_msgs)
            }
        })*
    }
    .into()
}
