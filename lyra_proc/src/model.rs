use std::ops::Deref;

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Result, Token,
};

pub struct Args(pub(super) Vec<Ident>);

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let vars = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        Ok(Self(vars.into_iter().collect()))
    }
}

impl Deref for Args {
    type Target = Vec<Ident>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
