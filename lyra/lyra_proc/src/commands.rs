use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

use crate::models::Args;

pub fn impl_out(input: &Expr) -> TokenStream {
    quote!(
        ctx.respond(#input).await?;
        return Ok(());
    )
    .into()
}

pub fn impl_err(input: &Expr) -> TokenStream {
    quote!(
        ctx.ephem(#input).await?;
        return Ok(());
    )
    .into()
}

pub fn impl_check(args: &Args) -> TokenStream {
    args.0
        .iter()
        .fold(quote!(), |c, a| match a.to_string().as_str() {
            "Guild" => quote!(
                #c

                let Some(guild_id) = ctx.guild_id() else {
                    return Err(Error::GuildOnly.into());
                };
            ),
            _ => panic!("unknown arg: {}", a),
        })
        .into()
}
