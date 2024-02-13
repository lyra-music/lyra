use quote::quote;

pub(super) fn impl_hook(fun: syn::ItemFn) -> proc_macro::TokenStream {
    let syn::ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = fun;

    let sig_span = syn::spanned::Spanned::span(&sig);
    let syn::Signature {
        asyncness,
        ident,
        mut inputs,
        output,
        ..
    } = sig;

    if asyncness.is_none() {
        return syn::Error::new(sig_span, "`async` keyword is missing")
            .to_compile_error()
            .into();
    }

    let output = match output {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, t) => quote!(#t),
    };

    populate_lifetime(&mut inputs);

    let result = quote! {
        #(#attrs)*
        #vis fn #ident<'fut>(#inputs) -> futures::future::BoxFuture<'fut, #output> {
            use futures::future::FutureExt;

            async move {
                #block
            }.boxed()
        }
    };

    result.into()
}

fn populate_lifetime(inputs: &mut syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>) {
    for input in inputs {
        if let syn::FnArg::Typed(kind) = input {
            if let syn::Type::Reference(ty) = &mut *kind.ty {
                ty.lifetime = Some(syn::Lifetime::new(
                    "'fut",
                    proc_macro::Span::call_site().into(),
                ));
            }
        }
    }
}
