use syn::spanned::Spanned;
use syn::{ExprPath, Meta, Result, parse::Parse};

use crate::common::make_uppercase_path;

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) unlogged: bool,
    pub(crate) temporary: bool,
    pub(crate) inherits: Option<String>,
    pub(crate) tablespace: Option<String>,
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
                                    attrs.marker_exprs.push(make_uppercase_path(ident, "NAME"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "Expected a string literal for 'NAME'",
                                ));
                            }
                            "INHERITS" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.inherits = Some(str_lit.value());
                                    attrs
                                        .marker_exprs
                                        .push(make_uppercase_path(ident, "INHERITS"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "Expected a string literal for 'INHERITS'",
                                ));
                            }
                            "TABLESPACE" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.tablespace = Some(str_lit.value());
                                    attrs
                                        .marker_exprs
                                        .push(make_uppercase_path(ident, "TABLESPACE"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "Expected a string literal for 'TABLESPACE'",
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
                            "UNLOGGED" => {
                                attrs.unlogged = true;
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "UNLOGGED"));
                                continue;
                            }
                            "TEMPORARY" => {
                                attrs.temporary = true;
                                attrs
                                    .marker_exprs
                                    .push(make_uppercase_path(ident, "TEMPORARY"));
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
                 Supported attributes (case-insensitive):\n\
                 - NAME: Custom table name (e.g., #[PostgresTable(NAME = \"custom_name\")])\n\
                 - UNLOGGED: Create UNLOGGED table (e.g., #[PostgresTable(UNLOGGED)])\n\
                 - TEMPORARY: Create TEMPORARY table (e.g., #[PostgresTable(TEMPORARY)])\n\
                 - INHERITS: Inherit from parent table (e.g., #[PostgresTable(INHERITS = \"parent_table\")])\n\
                 - TABLESPACE: Specify tablespace (e.g., #[PostgresTable(TABLESPACE = \"my_tablespace\")])\n\
                 See: https://www.postgresql.org/docs/current/sql-createtable.html",
            ));
        }
        Ok(attrs)
    }
}
