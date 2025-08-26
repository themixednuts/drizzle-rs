use syn::spanned::Spanned;
use syn::{Meta, Result, parse::Parse};

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) unlogged: bool,
    pub(crate) temporary: bool,
    pub(crate) if_not_exists: bool,
    pub(crate) inherits: Option<String>,
    pub(crate) tablespace: Option<String>,
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
                Meta::NameValue(nv) if nv.path.is_ident("inherits") => {
                    if let syn::Expr::Lit(lit) = nv.clone().value
                        && let syn::Lit::Str(str_lit) = lit.lit
                    {
                        attrs.inherits = Some(str_lit.value());
                        continue;
                    }
                    return Err(syn::Error::new(
                        nv.span(),
                        "Expected a string literal for 'inherits'",
                    ));
                }
                Meta::NameValue(nv) if nv.path.is_ident("tablespace") => {
                    if let syn::Expr::Lit(lit) = nv.clone().value
                        && let syn::Lit::Str(str_lit) = lit.lit
                    {
                        attrs.tablespace = Some(str_lit.value());
                        continue;
                    }
                    return Err(syn::Error::new(
                        nv.span(),
                        "Expected a string literal for 'tablespace'",
                    ));
                }
                Meta::Path(path) if path.is_ident("unlogged") => attrs.unlogged = true,
                Meta::Path(path) if path.is_ident("temporary") => attrs.temporary = true,
                Meta::Path(path) if path.is_ident("if_not_exists") => attrs.if_not_exists = true,
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "Unrecognized table attribute.\n\
                         Supported attributes:\n\
                         - name: Custom table name (e.g., #[PostgresTable(name = \"custom_name\")])\n\
                         - unlogged: Create UNLOGGED table (e.g., #[PostgresTable(unlogged)])\n\
                         - temporary: Create TEMPORARY table (e.g., #[PostgresTable(temporary)])\n\
                         - if_not_exists: Add IF NOT EXISTS clause (e.g., #[PostgresTable(if_not_exists)])\n\
                         - inherits: Inherit from parent table (e.g., #[PostgresTable(inherits = \"parent_table\")])\n\
                         - tablespace: Specify tablespace (e.g., #[PostgresTable(tablespace = \"my_tablespace\")])\n\
                         See: https://www.postgresql.org/docs/current/sql-createtable.html",
                    ));
                }
            }
        }
        Ok(attrs)
    }
}