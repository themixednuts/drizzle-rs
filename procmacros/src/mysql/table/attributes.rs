use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{ExprPath, Ident, Meta, Result, Token, parse::Parse};

use crate::common::make_uppercase_path;
use crate::mysql::string_value;

#[derive(Default)]
pub struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) database: Option<String>,
    pub(crate) temporary: bool,
    pub(crate) engine: Option<String>,
    pub(crate) charset: Option<String>,
    pub(crate) collate: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) composite_foreign_keys: Vec<CompositeForeignKeyAttr>,
    pub(crate) unique_constraints: Vec<UniqueConstraintAttr>,
    pub(crate) check_constraints: Vec<CheckConstraintAttr>,
    pub(crate) marker_exprs: Vec<ExprPath>,
}

#[derive(Clone)]
pub struct CompositeForeignKeyAttr {
    pub(crate) source_columns: Vec<Ident>,
    pub(crate) target_table: Ident,
    pub(crate) target_columns: Vec<Ident>,
    pub(crate) on_delete: Option<String>,
    pub(crate) on_update: Option<String>,
}

#[derive(Clone)]
pub struct UniqueConstraintAttr {
    pub(crate) columns: Vec<Ident>,
    pub(crate) name: Option<String>,
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
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let table = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        let columns = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        if columns.is_empty() {
            return Err(input.error("references(...) requires target columns"));
        }
        Ok(Self {
            table,
            columns: columns.into_iter().collect(),
        })
    }
}

fn option_identifier(meta: &syn::MetaNameValue, name: &str) -> Result<String> {
    let value = string_value(meta, name)?;
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(syn::Error::new(
            meta.span(),
            format!("{name} must contain only ASCII letters, digits, or underscores"),
        ));
    }
    Ok(value)
}

fn action(value: String, span: proc_macro2::Span) -> Result<String> {
    match value.to_ascii_uppercase().replace('_', " ").as_str() {
        "CASCADE" => Ok("CASCADE".into()),
        "SET NULL" => Ok("SET NULL".into()),
        "RESTRICT" => Ok("RESTRICT".into()),
        "NO ACTION" => Ok("NO ACTION".into()),
        "SET DEFAULT" => Err(syn::Error::new(
            span,
            "InnoDB rejects SET DEFAULT referential actions",
        )),
        _ => Err(syn::Error::new(
            span,
            "expected CASCADE, SET_NULL, RESTRICT, or NO_ACTION",
        )),
    }
}

impl Parse for CompositeForeignKeyAttr {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        let mut source_columns: Option<Vec<Ident>> = None;
        let mut target = None;
        let mut on_delete = None;
        let mut on_update = None;
        for meta in metas {
            match meta {
                Meta::List(list) if list.path.is_ident("columns") => {
                    let cols = Punctuated::<Ident, Token![,]>::parse_terminated
                        .parse2(list.tokens.clone())?;
                    if cols.is_empty() {
                        return Err(syn::Error::new(list.span(), "columns(...) cannot be empty"));
                    }
                    source_columns = Some(cols.into_iter().collect());
                }
                Meta::List(list) if list.path.is_ident("references") => {
                    target = Some(syn::parse2::<ReferencesArg>(list.tokens)?)
                }
                Meta::NameValue(value) if value.path.is_ident("on_delete") => {
                    let span = value.span();
                    on_delete = Some(action(string_value(&value, "on_delete")?, span)?);
                }
                Meta::NameValue(value) if value.path.is_ident("on_update") => {
                    let span = value.span();
                    on_update = Some(action(string_value(&value, "on_update")?, span)?);
                }
                Meta::Path(path)
                    if path.is_ident("deferrable") || path.is_ident("initially_deferred") =>
                {
                    return Err(syn::Error::new(
                        path.span(),
                        "MySQL foreign keys are not deferrable",
                    ));
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "invalid MySQL FOREIGN_KEY argument",
                    ));
                }
            }
        }
        let source_columns =
            source_columns.ok_or_else(|| input.error("FOREIGN_KEY requires columns(...)"))?;
        let target = target
            .ok_or_else(|| input.error("FOREIGN_KEY requires references(Table, columns...)"))?;
        if source_columns.len() != target.columns.len() {
            return Err(input.error("source and target foreign-key column counts must match"));
        }
        Ok(Self {
            source_columns,
            target_table: target.table,
            target_columns: target.columns,
            on_delete,
            on_update,
        })
    }
}

impl Parse for UniqueConstraintAttr {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        let mut columns = Vec::new();
        let mut name = None;
        for meta in metas {
            match meta {
                Meta::List(list) if list.path.is_ident("columns") => columns
                    .extend(Punctuated::<Ident, Token![,]>::parse_terminated.parse2(list.tokens)?),
                Meta::NameValue(value)
                    if value.path.is_ident("name") || value.path.is_ident("NAME") =>
                {
                    name = Some(string_value(&value, "name")?)
                }
                Meta::Path(path)
                    if path.is_ident("deferrable")
                        || path.is_ident("initially_deferred")
                        || path.is_ident("nulls_not_distinct") =>
                {
                    return Err(syn::Error::new(
                        path.span(),
                        "this PostgreSQL UNIQUE option is not supported by MySQL",
                    ));
                }
                Meta::Path(path) => columns.push(path.get_ident().cloned().ok_or_else(|| {
                    syn::Error::new(path.span(), "UNIQUE columns must be identifiers")
                })?),
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "invalid MySQL UNIQUE argument",
                    ));
                }
            }
        }
        if columns.is_empty() {
            return Err(input.error("UNIQUE requires at least one column"));
        }
        Ok(Self { columns, name })
    }
}

impl Parse for CheckConstraintAttr {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let metas = input.parse_terminated(Meta::parse, Token![,])?;
        let mut name = None;
        let mut expr = None;
        for meta in metas {
            match meta {
                Meta::NameValue(value)
                    if value.path.is_ident("name") || value.path.is_ident("NAME") =>
                {
                    name = Some(string_value(&value, "name")?)
                }
                Meta::NameValue(value)
                    if value.path.is_ident("expr")
                        || value.path.is_ident("EXPR")
                        || value.path.is_ident("value")
                        || value.path.is_ident("VALUE") =>
                {
                    expr = Some(string_value(&value, "expr")?)
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "CHECK accepts name = \"...\" and expr = \"...\"",
                    ));
                }
            }
        }
        Ok(Self {
            name,
            expr: expr.ok_or_else(|| input.error("CHECK requires expr = \"...\""))?,
        })
    }
}

impl Parse for TableAttributes {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let mut attrs = Self::default();
        for meta in input.parse_terminated(Meta::parse, Token![,])? {
            let ident =
                meta.path().get_ident().cloned().ok_or_else(|| {
                    syn::Error::new(meta.span(), "expected a MySQL table attribute")
                })?;
            let upper = ident.to_string().to_ascii_uppercase();
            match meta {
                Meta::NameValue(value) => match upper.as_str() {
                    "NAME" => attrs.name = Some(string_value(&value, "NAME")?),
                    "DATABASE" | "SCHEMA" => {
                        attrs.database = Some(string_value(&value, "DATABASE")?)
                    }
                    "ENGINE" => attrs.engine = Some(option_identifier(&value, "ENGINE")?),
                    "CHARSET" | "DEFAULT_CHARSET" => {
                        attrs.charset = Some(option_identifier(&value, "CHARSET")?)
                    }
                    "COLLATE" => attrs.collate = Some(option_identifier(&value, "COLLATE")?),
                    "COMMENT" => attrs.comment = Some(string_value(&value, "COMMENT")?),
                    "INHERITS" | "TABLESPACE" => {
                        return Err(syn::Error::new(
                            value.span(),
                            format!("{upper} is PostgreSQL-only"),
                        ));
                    }
                    _ => {
                        return Err(syn::Error::new(
                            value.span(),
                            format!("unrecognized MySQL table attribute `{upper}`"),
                        ));
                    }
                },
                Meta::Path(path) => match upper.as_str() {
                    "TEMPORARY" => attrs.temporary = true,
                    "UNLOGGED" | "RLS" | "STRICT" | "WITHOUT_ROWID" => {
                        return Err(syn::Error::new(
                            path.span(),
                            format!("{upper} is not a MySQL table option"),
                        ));
                    }
                    _ => {
                        return Err(syn::Error::new(
                            path.span(),
                            format!("unrecognized MySQL table attribute `{upper}`"),
                        ));
                    }
                },
                Meta::List(list) => match upper.as_str() {
                    "FOREIGN_KEY" => attrs.composite_foreign_keys.push(syn::parse2(list.tokens)?),
                    "UNIQUE" => attrs.unique_constraints.push(syn::parse2(list.tokens)?),
                    "CHECK" => attrs.check_constraints.push(syn::parse2(list.tokens)?),
                    _ => {
                        return Err(syn::Error::new(
                            list.span(),
                            format!("unrecognized MySQL table attribute `{upper}`"),
                        ));
                    }
                },
            }
            let marker = if upper == "SCHEMA" {
                "DATABASE"
            } else {
                &upper
            };
            attrs.marker_exprs.push(make_uppercase_path(&ident, marker));
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
