use proc_macro::TokenStream;
use quote::{__private::TokenStream as QuoteTokenStream, quote};
use syn::{Data, DeriveInput, Fields, FieldsUnnamed, Ident, Path, Type, TypePath, Variant};

const GUILD_STR: &str = "Guild";

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
    root_name_aware_path: &Path,
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
            let impl_for_inner = sub_cmd_inner.map_or_else(
                || quote!(),
                |inner| {
                    quote! {
                        impl #root_name_aware_path for #inner {
                            const ROOT_NAME: &'static str = #name::ROOT_NAME;
                            const PARENT_NAME: ::std::option::Option<&'static str>
                                = ::std::option::Option::Some(#name::NAME);
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
                    impl #root_name_aware_path for #sub_cmd {
                        const ROOT_NAME: &'static str = #name::ROOT_NAME;
                        const PARENT_NAME: ::std::option::Option<&'static str>
                            = ::std::option::Option::Some(#name::NAME);
                    }
                    #impl_for_inner
                },
            )
        }
        _ => panic!("the field must be a path"),
    }
}

pub fn impl_bot_command_group(input: &DeriveInput, guild: bool) -> TokenStream {
    let name = &input.ident;
    let data = &input.data;

    let root_name_aware_path =
        syn::parse_str::<Path>("crate::command::model::CommandStructureAware")
            .expect("path is valid");
    let (sub_cmd_matches, impls_resolved_command_data) = match data {
        Data::Enum(data) => {
            data.variants
                .iter()
                .fold((quote!(), quote!()), |c, v| match v.fields {
                    Fields::Unnamed(ref fields) => {
                        declare_commands(fields, v, c, name, &root_name_aware_path)
                    }
                    _ => panic!("all fields must be unnamed"),
                })
        }
        _ => panic!("this can only be derived from an enum"),
    };

    let guild_str = if guild { GUILD_STR } else { Default::default() };
    let bot_slash_command_path = syn::parse_str::<Path>(&format!(
        "crate::command::model::Bot{guild_str}SlashCommand"
    ))
    .expect("path is valid");
    let slash_ctx_path =
        syn::parse_str::<Path>(&format!("crate::command::model::{guild_str}SlashCmdCtx"))
            .expect("path is valid");
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

pub fn impl_bot_autocomplete_group(input: &DeriveInput, guild: bool) -> TokenStream {
    let name = &input.ident;
    let data = &input.data;

    let sub_autocomplete_matches = match data {
        Data::Enum(data) => data.variants.iter().fold(quote!(), |c, v| match v.fields {
            Fields::Unnamed(ref fields) => declare_autocompletes(fields, v, &c),
            _ => panic!("all fields must be unnamed"),
        }),
        _ => panic!("this can only be derived from an enum"),
    };

    let guild_str = if guild { GUILD_STR } else { Default::default() };
    let bot_autocomplete_path = syn::parse_str::<Path>(&format!(
        "crate::command::model::Bot{guild_str}Autocomplete"
    ))
    .expect("path is valid");
    let autocomplete_ctx_path = syn::parse_str::<Path>(&format!(
        "crate::command::model::{guild_str}AutocompleteCtx"
    ))
    .expect("path is valid");
    let result_path = syn::parse_str::<Path>(&format!(
        "crate::error::command::{guild_str}AutocompleteResult"
    ))
    .expect("path is valid");
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
