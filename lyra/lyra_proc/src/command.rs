use proc_macro::TokenStream;
use quote::{__private::TokenStream as QuoteTokenStream, quote};
use syn::{Data, DeriveInput, Fields, FieldsUnnamed, Ident, PathSegment, Type, TypePath, Variant};

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
            let sub_cmd_inner = unwrap(type_path, "Box");
            let impl_for_inner = match sub_cmd_inner {
                Some(inner) => quote! {
                    impl CommandInfoAware for #inner {
                        fn name() -> &'static str {
                            #name::name()
                        }
                    }
                },
                None => quote! {},
            };
            (
                quote! {
                    #sub_cmd_match
                    Self::#v_ident(sub_cmd) => sub_cmd.run(ctx).await,
                },
                quote! {
                    #impl_resolved_command_data
                    impl CommandInfoAware for #sub_cmd {
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

pub(super) fn impl_lyra_command_group(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let data = &input.data;

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
        impl BotSlashCommand for #name {
            async fn run(self, ctx: Ctx<SlashCommand>) -> CommandResult {
                match self {
                    #sub_cmd_matches
                }
            }
        }

        #impls_resolved_command_data
    }
    .into()
}
