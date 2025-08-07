///! make this utils for way to create builders for
use syn::Ident;

pub(crate) struct ImplBlockFor<'a> {
    ident: &'a Ident,
}

pub(crate) struct ImplBlock<'a> {
    generics: Vec<&'static str>,
    ident: &'a Ident,
    r#for: ImplBlockFor<'a>,
}
