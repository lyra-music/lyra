use proc_macro::TokenStream;
use quote::{__private::TokenStream as QuoteTokenStream, quote};
use syn::{Data, DeriveInput, Fields, FieldsUnnamed, Ident, PathSegment, Type, TypePath, Variant};

use crate::models::Args;

pub(super) fn impl_check(checks: &Args) -> TokenStream {
    checks
        .iter()
        .fold(quote! {}, |c, a| match a.to_string().as_str() {
            "..." => quote!(
                #c
                todo!()
            ),
            _ => panic!("unknown arg: {}", a),
        })
        .into()
}

fn unwrap(type_path: &TypePath, from: impl Into<String>) -> Option<&PathSegment> {
    match type_path.path.segments.last() {
        Some(segment) if segment.ident == from.into() => match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => match &args.args[0]
            {
                syn::GenericArgument::Type(Type::Path(inner_type_path)) => {
                    match inner_type_path.path.segments.last() {
                        Some(inner_segment) => Some(&inner_segment),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn process(
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
            let impl_for_inner = match unwrap(type_path, "Box") {
                Some(inner) => quote! {
                    impl ResolvedCommandInfo for #inner {
                        fn id() -> Id<CommandMarker> {
                            #name::id()
                        }
                        fn name() -> String {
                            #name::name()
                        }
                    }
                },
                None => quote! {},
            };
            (
                quote! {
                    #sub_cmd_match
                    Self::#v_ident(sub_cmd) => sub_cmd.execute(ctx).await,
                },
                quote! {
                    #impl_resolved_command_data
                    impl ResolvedCommandInfo for #sub_cmd {
                        fn id() -> Id<CommandMarker> {
                            #name::id()
                        }
                        fn name() -> String {
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

pub(super) fn impl_lyra_command_group(input: &DeriveInput) -> TokenStream {
    let ref name = input.ident;
    let ref data = input.data;

    let (sub_cmd_matches, impls_resolved_command_data) = match data {
        Data::Enum(data) => {
            data.variants
                .iter()
                .fold((quote! {}, quote! {}), |c, v| match v.fields {
                    Fields::Unnamed(ref fields) => process(fields, v, c, name),
                    _ => panic!("all fields must be unnamed"),
                })
        }
        _ => panic!("this can only be derived from an enum"),
    };

    quote! {
        #[async_trait]
        impl LyraCommand for #name {
            async fn execute(self, ctx: Context<App>) -> Result<()> {
                match self {
                    #sub_cmd_matches
                }
            }
        }

        #impls_resolved_command_data
    }
    .into()
}
