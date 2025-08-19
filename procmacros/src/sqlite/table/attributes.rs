use syn::spanned::Spanned;
use syn::{Meta, Result, parse::Parse};

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) strict: bool,
    pub(crate) without_rowid: bool,
}

impl Parse for TableAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = TableAttributes::default();
        let metas = input.parse_terminated(Meta::parse, syn::Token![,])?;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    if let syn::Expr::Lit(lit) = nv.clone().value
                        && let syn::Lit::Str(str_lit) = lit.lit
                    {
                        attrs.name = Some(str_lit.value());
                        continue;
                    }
                    return Err(syn::Error::new(
                        nv.span(),
                        "Expected a string literal for 'name'",
                    ));
                }
                Meta::Path(path) if path.is_ident("strict") => attrs.strict = true,
                Meta::Path(path) if path.is_ident("without_rowid") => attrs.without_rowid = true,
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "Unrecognized table attribute.\n\
                         Supported attributes:\n\
                         - name: Custom table name (e.g., #[table(name = \"custom_name\")])\n\
                         - strict: Enable STRICT mode (e.g., #[table(strict)])\n\
                         - without_rowid: Use WITHOUT ROWID optimization (e.g., #[table(without_rowid)])\n\
                         See: https://sqlite.org/lang_createtable.html",
                    ));
                }
            }
        }
        Ok(attrs)
    }
}
