use proc_macro::TokenStream;
use quote::{__private::TokenStream as QuoteTokenStream, quote};
use syn::{Data, DeriveInput, Fields, FieldsUnnamed, Ident, Path, Type, TypePath, Variant};

fn unwrap(type_path: &TypePath, from: impl Into<String>) -> Option<&Path> {
    match type_path.path.segments.last() {
        Some(segment) if segment.ident == from.into() => match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => match &args.args[0]
            {
                syn::GenericArgument::Type(Type::Path(inner_type_path)) => {
                    Some(&inner_type_path.path)
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn declare_commands(
    fields: &FieldsUnnamed,
    v: &Variant,
    c: (QuoteTokenStream, QuoteTokenStream),
    name: &Ident,
) -> (QuoteTokenStream, QuoteTokenStream) {
    let sub_cmd = fields
        .unnamed
        .first()
        .expect("variant must have exactly one unnamed field");
    let v_ident = &v.ident;
    match sub_cmd.ty {
        Type::Path(ref type_path) => {
            let (sub_cmd_match, impl_resolved_command_data) = c;
            let sub_cmd_inner = unwrap(type_path, "Box");
            let command_info_aware_path =
                syn::parse_str::<Path>("crate::command::model::CommandInfoAware")
                    .expect("path is valid");
            let impl_for_inner = sub_cmd_inner.map_or_else(
                || quote!(),
                |inner| {
                    quote! {
                        impl #command_info_aware_path for #inner {
                            fn name() -> &'static str {
                                #name::name()
                            }
                        }
                    }
                },
            );

            (
                quote! {
                    #sub_cmd_match
                    Self::#v_ident(sub_cmd) => sub_cmd.run(ctx).await,
                },
                quote! {
                    #impl_resolved_command_data
                    impl #command_info_aware_path for #sub_cmd {
                        fn name() -> &'static str {
                            #name::name()
                        }
                    }
                    #impl_for_inner
                },
            )
        }
        _ => panic!("the field must be a path"),
    }
}

pub fn impl_bot_command_group(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let data = &input.data;

    let (sub_cmd_matches, impls_resolved_command_data) = match data {
        Data::Enum(data) => {
            data.variants
                .iter()
                .fold((quote!(), quote!()), |c, v| match v.fields {
                    Fields::Unnamed(ref fields) => declare_commands(fields, v, c, name),
                    _ => panic!("all fields must be unnamed"),
                })
        }
        _ => panic!("this can only be derived from an enum"),
    };

    let bot_slash_command_path =
        syn::parse_str::<Path>("crate::command::model::BotSlashCommand").expect("path is valid");
    let slash_ctx_path =
        syn::parse_str::<Path>("crate::command::model::SlashCtx").expect("path is valid");
    let result_path =
        syn::parse_str::<Path>("crate::error::command::Result").expect("path is valid");
    quote! {
        impl #bot_slash_command_path for #name {
            async fn run(self, ctx: #slash_ctx_path) -> #result_path {
                match self {
                    #sub_cmd_matches
                }
            }
        }

        #impls_resolved_command_data
    }
    .into()
}

fn declare_autocompletes(
    fields: &FieldsUnnamed,
    v: &Variant,
    sub_autocomplete_match: &QuoteTokenStream,
) -> QuoteTokenStream {
    let sub_cmd = fields
        .unnamed
        .first()
        .expect("variant must have exactly one unnamed field");
    let v_ident = &v.ident;
    match sub_cmd.ty {
        Type::Path(_) => {
            quote! {
                #sub_autocomplete_match
                Self::#v_ident(sub_cmd) => sub_cmd.execute(ctx).await,
            }
        }
        _ => panic!("the field must be a path"),
    }
}

pub fn impl_bot_autocomplete_group(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let data = &input.data;

    let sub_autocomplete_matches = match data {
        Data::Enum(data) => data.variants.iter().fold(quote!(), |c, v| match v.fields {
            Fields::Unnamed(ref fields) => declare_autocompletes(fields, v, &c),
            _ => panic!("all fields must be unnamed"),
        }),
        _ => panic!("this can only be derived from an enum"),
    };

    let bot_autocomplete_path =
        syn::parse_str::<Path>("crate::command::model::BotAutocomplete").expect("path is valid");
    let autocomplete_ctx_path =
        syn::parse_str::<Path>("crate::command::model::AutocompleteCtx").expect("path is valid");
    let result_path =
        syn::parse_str::<Path>("crate::error::command::AutocompleteResult").expect("path is valid");
    quote! {
        impl #bot_autocomplete_path for #name {
            async fn execute(self, ctx: #autocomplete_ctx_path) -> #result_path {
                match self {
                    #sub_autocomplete_matches
                }
            }
        }
    }
    .into()
}
