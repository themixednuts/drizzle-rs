use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{ExprPath, Ident, Meta, Result, Token, parse::Parse};

use crate::common::make_uppercase_path;

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) strict: bool,
    pub(crate) without_rowid: bool,
    pub(crate) crate_name: Option<String>,
    pub(crate) composite_foreign_keys: Vec<CompositeForeignKeyAttr>,
    /// Original marker paths for IDE hover documentation
    pub(crate) marker_exprs: Vec<ExprPath>,
}

#[derive(Clone)]
pub(crate) struct CompositeForeignKeyAttr {
    pub(crate) source_columns: Vec<Ident>,
    pub(crate) target_table: Ident,
    pub(crate) target_columns: Vec<Ident>,
    pub(crate) on_delete: Option<String>,
    pub(crate) on_update: Option<String>,
}

struct ReferencesArg {
    table: Ident,
    columns: Vec<Ident>,
}

impl Parse for ReferencesArg {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let table: Ident = input.parse()?;
        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
        }
        let columns: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(input)?;
        if columns.is_empty() {
            return Err(syn::Error::new(
                table.span(),
                "references(...) must include at least one target column",
            ));
        }
        Ok(Self {
            table,
            columns: columns.into_iter().collect(),
        })
    }
}

impl Parse for CompositeForeignKeyAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        let mut source_columns: Option<Vec<Ident>> = None;
        let mut target_table: Option<Ident> = None;
        let mut target_columns: Option<Vec<Ident>> = None;
        let mut on_delete: Option<String> = None;
        let mut on_update: Option<String> = None;

        for meta in metas {
            match meta {
                Meta::List(list) if list.path.is_ident("columns") => {
                    let cols: Punctuated<Ident, Token![,]> =
                        Punctuated::<Ident, Token![,]>::parse_terminated
                            .parse2(list.tokens.clone())?;
                    if cols.is_empty() {
                        return Err(syn::Error::new(
                            list.span(),
                            "columns(...) must include at least one source column",
                        ));
                    }
                    source_columns = Some(cols.into_iter().collect());
                }
                Meta::List(list) if list.path.is_ident("references") => {
                    let r: ReferencesArg = syn::parse2(list.tokens.clone())?;
                    target_table = Some(r.table);
                    target_columns = Some(r.columns);
                }
                Meta::NameValue(nv) if nv.path.is_ident("on_delete") => {
                    if let syn::Expr::Lit(lit) = &nv.value
                        && let syn::Lit::Str(s) = &lit.lit
                    {
                        on_delete = Some(s.value());
                    } else {
                        return Err(syn::Error::new(
                            nv.span(),
                            "on_delete must be a string literal",
                        ));
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("on_update") => {
                    if let syn::Expr::Lit(lit) = &nv.value
                        && let syn::Lit::Str(s) = &lit.lit
                    {
                        on_update = Some(s.value());
                    } else {
                        return Err(syn::Error::new(
                            nv.span(),
                            "on_update must be a string literal",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "FOREIGN_KEY expects columns(...), references(...), optional on_delete/on_update",
                    ));
                }
            }
        }

        let source_columns = source_columns.ok_or_else(|| {
            syn::Error::new(input.span(), "FOREIGN_KEY missing columns(...) argument")
        })?;
        let target_table = target_table.ok_or_else(|| {
            syn::Error::new(input.span(), "FOREIGN_KEY missing references(...) argument")
        })?;
        let target_columns = target_columns
            .ok_or_else(|| syn::Error::new(input.span(), "FOREIGN_KEY missing target columns"))?;

        if source_columns.len() != target_columns.len() {
            return Err(syn::Error::new(
                input.span(),
                "FOREIGN_KEY source and target column counts must match",
            ));
        }

        Ok(Self {
            source_columns,
            target_table,
            target_columns,
            on_delete,
            on_update,
        })
    }
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
                Meta::List(list) => {
                    if let Some(ident) = list.path.get_ident() {
                        let ident_upper = ident.to_string().to_ascii_uppercase();
                        if ident_upper == "FOREIGN_KEY" {
                            let fk: CompositeForeignKeyAttr = syn::parse2(list.tokens.clone())?;
                            attrs.composite_foreign_keys.push(fk);
                            continue;
                        }
                    }
                }
            }
            return Err(syn::Error::new(
                meta.span(),
                "Unrecognized table attribute.\n\
                 Supported attributes:\n\
                 - name/NAME: Custom table name (e.g., #[SQLiteTable(name = \"custom_name\")])\n\
                 - strict/STRICT: Enable STRICT mode (e.g., #[SQLiteTable(strict)])\n\
                 - without_rowid/WITHOUT_ROWID: Use WITHOUT ROWID optimization\n\
                 - FOREIGN_KEY(...): Composite FK (e.g., #[SQLiteTable(FOREIGN_KEY(columns(a,b), references(Parent,id_a,id_b)))])\n\
                 See: https://sqlite.org/lang_createtable.html",
            ));
        }
        Ok(attrs)
    }
}
