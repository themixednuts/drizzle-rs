use syn::spanned::Spanned;
use syn::{ExprPath, Meta, Result, parse::Parse};

use crate::common::make_uppercase_path;

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) strict: bool,
    pub(crate) without_rowid: bool,
    pub(crate) crate_name: Option<String>,
    /// Original marker paths for IDE hover documentation
    pub(crate) marker_exprs: Vec<ExprPath>,
}

impl Parse for TableAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = TableAttributes::default();
        let metas = input.parse_terminated(Meta::parse, syn::Token![,])?;

        for meta in metas {
            match &meta {
                Meta::NameValue(nv) => {
                    if let Some(ident) = nv.path.get_ident() {
                        let ident_str = ident.to_string();
                        let upper = ident_str.to_ascii_uppercase();
                        match upper.as_str() {
                            "NAME" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.name = Some(str_lit.value());
                                    // Store UPPERCASE path with original span for IDE hover
                                    attrs.marker_exprs.push(make_uppercase_path(ident, "NAME"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "Expected a string literal for 'NAME'",
                                ));
                            }
                            "CRATE" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.crate_name = Some(str_lit.value());
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "Expected a string literal for 'crate'",
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                Meta::Path(path) => {
                    if let Some(ident) = path.get_ident() {
                        let ident_str = ident.to_string();
                        let upper = ident_str.to_ascii_uppercase();
                        match upper.as_str() {
                            "STRICT" => {
                                attrs.strict = true;
                                // Store UPPERCASE path with original span for IDE hover
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "STRICT"));
                                continue;
                            }
                            "WITHOUT_ROWID" => {
                                attrs.without_rowid = true;
                                // Store UPPERCASE path with original span for IDE hover
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "WITHOUT_ROWID"));
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            return Err(syn::Error::new(
                meta.span(),
                "Unrecognized table attribute.\n\
                 Supported attributes:\n\
                 - name/NAME: Custom table name (e.g., #[SQLiteTable(name = \"custom_name\")])\n\
                 - strict/STRICT: Enable STRICT mode (e.g., #[SQLiteTable(strict)])\n\
                 - without_rowid/WITHOUT_ROWID: Use WITHOUT ROWID optimization\n\
                 See: https://sqlite.org/lang_createtable.html",
            ));
        }
        Ok(attrs)
    }
}
