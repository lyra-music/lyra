use heck::ToPascalCase;
use proc_macro::TokenStream;
use quote::quote;
use serde::Deserialize;
use syn::Ident;

#[derive(Deserialize)]
struct Presets {
    equaliser: Vec<Equalisers>,
}

#[derive(Deserialize)]
struct Equalisers {
    name: String,
    gains: [f64; 15],
}

pub fn impl_read_equaliser_presets_as(ty: &Ident) -> TokenStream {
    let presets_str = include_str!("../../preset/equalisers.toml");
    let presets_toml = toml::from_str::<Presets>(presets_str)
        .unwrap_or_else(|e| panic!("parsing equalisers.toml failed: {e:?}"));

    let preset_names_strs = presets_toml.equaliser.iter().map(|e| e.name.as_str());
    let preset_names_idents = presets_toml
        .equaliser
        .iter()
        .map(|e| {
            Ident::new(
                &e.name.replace("R&B", "Rnb").to_pascal_case(),
                proc_macro::Span::call_site().into(),
            )
        })
        .collect::<Box<_>>();
    let preset_gains = presets_toml.equaliser.iter().map(|e| {
        let gains = e.gains;
        quote!([#(#gains,)*])
    });

    quote! {
        #[derive(::twilight_interactions::command::CommandOption, ::twilight_interactions::command::CreateOption)]
        enum #ty {
            #(
                #[option(name = #preset_names_strs, value = #preset_names_strs)]
                #preset_names_idents,
            )*
        }

        impl #ty {
            const fn gains(&self) -> [f64; 15] {
                match self {
                    #(Self::#preset_names_idents => #preset_gains,)*
                }
            }
        }
    }
    .into()
}
