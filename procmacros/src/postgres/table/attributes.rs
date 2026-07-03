use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{ExprPath, Ident, Meta, Result, Token, parse::Parse};

use crate::common::make_uppercase_path;

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) schema: Option<String>,
    pub(crate) unlogged: bool,
    pub(crate) temporary: bool,
    pub(crate) inherits: Option<String>,
    pub(crate) tablespace: Option<String>,
    pub(crate) rls: bool,
    pub(crate) composite_foreign_keys: Vec<CompositeForeignKeyAttr>,
    pub(crate) unique_constraints: Vec<UniqueConstraintAttr>,
    pub(crate) check_constraints: Vec<CheckConstraintAttr>,
    /// Original marker paths for IDE hover documentation
    pub(crate) marker_exprs: Vec<ExprPath>,
}

#[derive(Clone)]
pub struct CompositeForeignKeyAttr {
    pub(crate) source_columns: Vec<Ident>,
    pub(crate) target_table: Ident,
    pub(crate) target_columns: Vec<Ident>,
    pub(crate) on_delete: Option<String>,
    pub(crate) on_update: Option<String>,
    pub(crate) deferrable: bool,
    pub(crate) initially_deferred: bool,
}

#[derive(Clone)]
pub struct UniqueConstraintAttr {
    pub(crate) columns: Vec<Ident>,
    pub(crate) name: Option<String>,
    pub(crate) deferrable: bool,
    pub(crate) initially_deferred: bool,
}

#[derive(Clone)]
pub struct CheckConstraintAttr {
    pub(crate) name: Option<String>,
    pub(crate) expr: String,
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
        let mut deferrable = false;
        let mut initially_deferred = false;

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
                Meta::Path(path) if path.is_ident("deferrable") => {
                    deferrable = true;
                }
                Meta::Path(path) if path.is_ident("initially_deferred") => {
                    deferrable = true;
                    initially_deferred = true;
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "unrecognized FOREIGN_KEY argument; expected columns(...), references(...), on_delete, on_update, deferrable, or initially_deferred",
                    ));
                }
            }
        }

        let source_columns = source_columns.ok_or_else(|| {
            syn::Error::new(input.span(), "FOREIGN_KEY requires a columns(...) argument")
        })?;
        let target_table = target_table.ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "FOREIGN_KEY requires a references(...) argument",
            )
        })?;
        let target_columns = target_columns.ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "FOREIGN_KEY requires target columns in references(Table, col1, col2)",
            )
        })?;

        if source_columns.len() != target_columns.len() {
            return Err(syn::Error::new(
                input.span(),
                "FOREIGN_KEY columns(...) and references(...) must have the same number of columns",
            ));
        }

        Ok(Self {
            source_columns,
            target_table,
            target_columns,
            on_delete,
            on_update,
            deferrable,
            initially_deferred,
        })
    }
}

impl Parse for UniqueConstraintAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        let mut columns_from_list: Option<Vec<Ident>> = None;
        let mut direct_columns = Vec::new();
        let mut name = None;
        let mut deferrable = false;
        let mut initially_deferred = false;

        for meta in metas {
            match meta {
                Meta::List(list)
                    if list
                        .path
                        .get_ident()
                        .is_some_and(|ident| ident.to_string().eq_ignore_ascii_case("columns")) =>
                {
                    let cols: Punctuated<Ident, Token![,]> =
                        Punctuated::<Ident, Token![,]>::parse_terminated
                            .parse2(list.tokens.clone())?;
                    if cols.is_empty() {
                        return Err(syn::Error::new(
                            list.span(),
                            "columns(...) must include at least one column",
                        ));
                    }
                    columns_from_list = Some(cols.into_iter().collect());
                }
                Meta::NameValue(nv) if nv.path.is_ident("name") || nv.path.is_ident("NAME") => {
                    if let syn::Expr::Lit(lit) = &nv.value
                        && let syn::Lit::Str(s) = &lit.lit
                    {
                        name = Some(s.value());
                    } else {
                        return Err(syn::Error::new(nv.span(), "name must be a string literal"));
                    }
                }
                Meta::Path(path) if path.is_ident("deferrable") => {
                    deferrable = true;
                }
                Meta::Path(path) if path.is_ident("initially_deferred") => {
                    deferrable = true;
                    initially_deferred = true;
                }
                Meta::Path(path) => {
                    if let Some(ident) = path.get_ident() {
                        direct_columns.push(ident.clone());
                    } else {
                        return Err(syn::Error::new(
                            path.span(),
                            "UNIQUE(...) columns must be identifiers",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "unrecognized UNIQUE argument; expected columns(...), name = \"...\", direct column identifiers, deferrable, or initially_deferred",
                    ));
                }
            }
        }

        let columns = columns_from_list.unwrap_or(direct_columns);
        if columns.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "UNIQUE requires at least one column",
            ));
        }

        Ok(Self {
            columns,
            name,
            deferrable,
            initially_deferred,
        })
    }
}

impl Parse for CheckConstraintAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        let mut name = None;
        let mut expr = None;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("name") || nv.path.is_ident("NAME") => {
                    if let syn::Expr::Lit(lit) = &nv.value
                        && let syn::Lit::Str(s) = &lit.lit
                    {
                        name = Some(s.value());
                    } else {
                        return Err(syn::Error::new(nv.span(), "name must be a string literal"));
                    }
                }
                Meta::NameValue(nv)
                    if nv.path.is_ident("expr")
                        || nv.path.is_ident("EXPR")
                        || nv.path.is_ident("value")
                        || nv.path.is_ident("VALUE") =>
                {
                    if let syn::Expr::Lit(lit) = &nv.value
                        && let syn::Lit::Str(s) = &lit.lit
                    {
                        expr = Some(s.value());
                    } else {
                        return Err(syn::Error::new(nv.span(), "expr must be a string literal"));
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "unrecognized CHECK argument; expected name = \"...\" or expr = \"...\"",
                    ));
                }
            }
        }

        let expr = expr.ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "CHECK requires expr = \"...\", e.g. CHECK(expr = \"score >= 0\")",
            )
        })?;

        Ok(Self { name, expr })
    }
}

impl Parse for TableAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = Self::default();
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
                                    "NAME requires a string literal, e.g. NAME = \"my_table\"",
                                ));
                            }
                            "SCHEMA" => {
                                if let syn::Expr::Lit(lit) = nv.clone().value
                                    && let syn::Lit::Str(str_lit) = lit.lit
                                {
                                    attrs.schema = Some(str_lit.value());
                                    attrs
                                        .marker_exprs
                                        .push(make_uppercase_path(ident, "SCHEMA"));
                                    continue;
                                }
                                return Err(syn::Error::new(
                                    nv.span(),
                                    "SCHEMA requires a string literal, e.g. SCHEMA = \"auth\"",
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
                                    "INHERITS requires a string literal, e.g. INHERITS = \"parent_table\"",
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
                                    "TABLESPACE requires a string literal, e.g. TABLESPACE = \"my_tablespace\"",
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
                            "RLS" => {
                                attrs.rls = true;
                                attrs.marker_exprs.push(make_uppercase_path(ident, "RLS"));
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
                            attrs
                                .marker_exprs
                                .push(make_uppercase_path(ident, "FOREIGN_KEY"));
                            continue;
                        }
                        if ident_upper == "UNIQUE" {
                            let unique: UniqueConstraintAttr = syn::parse2(list.tokens.clone())?;
                            attrs.unique_constraints.push(unique);
                            attrs
                                .marker_exprs
                                .push(make_uppercase_path(ident, "UNIQUE"));
                            continue;
                        }
                        if ident_upper == "CHECK" {
                            let check: CheckConstraintAttr = syn::parse2(list.tokens.clone())?;
                            attrs.check_constraints.push(check);
                            attrs.marker_exprs.push(make_uppercase_path(ident, "CHECK"));
                            continue;
                        }
                    }
                }
            }
            return Err(syn::Error::new(
                meta.span(),
                "unrecognized table attribute.\n\
                 Supported attributes (case-insensitive):\n\
                 - NAME: Custom table name (e.g., #[PostgresTable(NAME = \"custom_name\")])\n\
                 - SCHEMA: Custom schema name (e.g., #[PostgresTable(SCHEMA = \"auth\")])\n\
                 - UNLOGGED: Create UNLOGGED table (e.g., #[PostgresTable(UNLOGGED)])\n\
                 - TEMPORARY: Create TEMPORARY table (e.g., #[PostgresTable(TEMPORARY)])\n\
                 - INHERITS: Inherit from parent table (e.g., #[PostgresTable(INHERITS = \"parent_table\")])\n\
                 - TABLESPACE: Specify tablespace (e.g., #[PostgresTable(TABLESPACE = \"my_tablespace\")])\n\
                 - RLS: Enable row-level security (e.g., #[PostgresTable(RLS)])\n\
                 - FOREIGN_KEY(...): Composite FK (e.g., #[PostgresTable(FOREIGN_KEY(columns(a,b), references(Parent,id_a,id_b)))])\n\
                 - UNIQUE(...): Table-level unique constraint (e.g., #[PostgresTable(UNIQUE(columns(a,b)))])\n\
                 - CHECK(...): Table-level check constraint (e.g., #[PostgresTable(CHECK(expr = \"score >= 0\"))])\n\
                 See: https://www.postgresql.org/docs/current/sql-createtable.html",
            ));
        }
        Ok(attrs)
    }
}

impl crate::common::constraints::CompositeForeignKeyRef for CompositeForeignKeyAttr {
    fn target_table(&self) -> &Ident {
        &self.target_table
    }
    fn source_columns(&self) -> &[Ident] {
        &self.source_columns
    }
    fn target_columns(&self) -> &[Ident] {
        &self.target_columns
    }
}
