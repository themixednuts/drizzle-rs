use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::fmt::Write as _;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{
    Error, Expr, ExprPath, Field, GenericArgument, Ident, Lit, Meta, PathArguments, Result, Token,
    Type,
};

use crate::common::{
    Constraint, is_option_type, make_uppercase_path, option_inner_type,
    references_required_message, type_is_array_char, type_is_array_string, type_is_array_u8,
    type_is_arrayvec_u8, type_is_bool, type_is_datetime_tz, type_is_float, type_is_int,
    type_is_json_value, type_is_naive_date, type_is_naive_datetime, type_is_naive_time,
    type_is_offset_datetime, type_is_primitive_date_time, type_is_string_like, type_is_time_date,
    type_is_time_time, type_is_uuid, type_is_vec_u8,
};
use drizzle_types::mysql::{MySQLType, TypeCategory as MySQLRustTypeCategory};

use super::escape_string as escape_mysql_string;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategory {
    String,
    Blob,
    Enum,
    Set,
    Other,
}

#[derive(Debug, Clone)]
pub enum MySQLDefault {
    Literal(String),
    RawSql(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedColumn {
    pub expression: String,
    pub stored: bool,
}

#[derive(Debug, Clone)]
pub struct MySQLReference {
    pub table: Ident,
    pub column: Ident,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub ident: Ident,
    pub field_type: Type,
    pub base_type: Type,
    pub column_name: String,
    pub column_type: MySQLType,
    pub type_args: Vec<u16>,
    pub sql_definition: String,
    pub is_nullable: bool,
    pub is_enum: bool,
    pub is_set: bool,
    pub is_auto_increment: bool,
    pub generated_column: Option<GeneratedColumn>,
    pub default: Option<MySQLDefault>,
    pub default_fn: Option<TokenStream>,
    pub check_constraint: Option<String>,
    pub foreign_key: Option<MySQLReference>,
    pub relation_name: Option<String>,
    pub has_default: bool,
    pub marker_exprs: Vec<ExprPath>,
    pub constraint: Constraint,
    pub charset: Option<String>,
    pub collate: Option<String>,
    pub on_update: Option<String>,
    /// MySQL column `COMMENT`, retained as structured schema metadata for
    /// migration snapshots as well as rendered DDL.
    pub comment: Option<String>,
}

#[derive(Default)]
struct ParsedColumn {
    explicit_type: Option<MySQLType>,
    type_args: Vec<u16>,
    is_enum: bool,
    is_set: bool,
    is_auto_increment: bool,
    primary: bool,
    unique: bool,
    not_null: bool,
    default: Option<MySQLDefault>,
    default_fn: Option<TokenStream>,
    generated: Option<GeneratedColumn>,
    check: Option<String>,
    reference: Option<MySQLReference>,
    relation_name: Option<String>,
    reference_on_delete: Option<String>,
    reference_on_update: Option<String>,
    name: Option<String>,
    charset: Option<String>,
    collate: Option<String>,
    on_update: Option<String>,
    comment: Option<String>,
    marker_exprs: Vec<ExprPath>,
}

struct GeneratedArgs {
    stored: bool,
    expression: String,
}

impl Parse for GeneratedArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let kind: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let expression: syn::LitStr = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("generated(...) accepts exactly two arguments"));
        }
        let kind_name = kind.to_string();
        let stored = if kind_name.eq_ignore_ascii_case("stored") {
            true
        } else if kind_name.eq_ignore_ascii_case("virtual") {
            false
        } else {
            return Err(Error::new_spanned(
                kind,
                "expected `stored` or `virtual` in generated(kind, \"expression\")",
            ));
        };
        Ok(Self {
            stored,
            expression: expression.value(),
        })
    }
}

impl FieldInfo {
    pub fn from_field(field: &Field, is_composite_pk: bool) -> Result<Self> {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(field, "MySQLTable requires named fields"))?;
        let field_type = field.ty.clone();
        let base_type = option_inner_type(&field_type)
            .cloned()
            .unwrap_or_else(|| field_type.clone());
        let mut parsed = parse_column_attrs(field)?;
        if parsed.relation_name.is_some() && parsed.reference.is_none() {
            return Err(Error::new_spanned(
                field,
                crate::common::relation_requires_references_message(),
            ));
        }
        let rust_category = mysql_rust_category(&base_type);

        let column_type = if parsed.is_enum {
            if parsed.explicit_type.is_some() {
                return Err(Error::new_spanned(
                    field,
                    "MySQL inline enums use #[column(ENUM)] without a second SQL type override",
                ));
            }
            MySQLType::enum_values(Vec::<String>::new())
        } else if parsed.is_set {
            parsed.explicit_type.take().ok_or_else(|| {
                Error::new_spanned(field, "SET requires values: #[column(SET(\"a\", \"b\"))]")
            })?
        } else if let Some(explicit) = parsed.explicit_type.take() {
            validate_explicit_signedness(field, rust_category, &explicit)?;
            explicit
        } else {
            let (column_type, inferred_args) = infer_mysql_type(field, &base_type, rust_category)?;
            parsed.type_args = inferred_args;
            column_type
        };

        validate_type_args(field, &column_type, &parsed.type_args)?;

        let is_nullable = is_option_type(&field_type);
        if parsed.not_null && is_nullable {
            return Err(Error::new_spanned(
                field,
                "Option<T> cannot be combined with NOT_NULL",
            ));
        }
        if parsed.primary && is_nullable {
            return Err(Error::new_spanned(
                field,
                "MySQL primary-key fields cannot be nullable; remove Option<T>",
            ));
        }
        if (parsed.charset.is_some() || parsed.collate.is_some())
            && !supports_character_options(&column_type)
        {
            return Err(Error::new_spanned(
                field,
                "CHARACTER_SET/CHARSET and COLLATE apply only to MySQL character, text, ENUM, or SET columns",
            ));
        }
        if parsed.on_update.is_some()
            && !matches!(column_type, MySQLType::Datetime | MySQLType::Timestamp)
        {
            return Err(Error::new_spanned(
                field,
                "column ON_UPDATE applies only to MySQL DATETIME or TIMESTAMP columns",
            ));
        }
        if parsed.is_auto_increment {
            if !column_type.supports_auto_increment() {
                return Err(Error::new_spanned(
                    field,
                    "AUTO_INCREMENT requires a signed or unsigned MySQL integer column",
                ));
            }
            if parsed.generated.is_some() || parsed.default.is_some() {
                return Err(Error::new_spanned(
                    field,
                    "AUTO_INCREMENT cannot be combined with DEFAULT or GENERATED",
                ));
            }
            if !(parsed.primary || parsed.unique) {
                return Err(Error::new_spanned(
                    field,
                    "MySQL requires an AUTO_INCREMENT column to be keyed; add PRIMARY or UNIQUE",
                ));
            }
        }
        if parsed.generated.is_some()
            && (parsed.default.is_some()
                || parsed.default_fn.is_some()
                || parsed.on_update.is_some())
        {
            return Err(Error::new_spanned(
                field,
                "GENERATED columns cannot use DEFAULT, DEFAULT_FN, or ON_UPDATE",
            ));
        }
        if parsed.default.is_some() && parsed.default_fn.is_some() {
            return Err(Error::new_spanned(
                field,
                "DEFAULT/DEFAULT_SQL and DEFAULT_FN are mutually exclusive",
            ));
        }
        if matches!(column_type, MySQLType::Json) {
            parsed.marker_exprs.push(parse_quote_path("JSON"));
        }

        let constraint = Constraint::from_flags(parsed.primary, parsed.unique, is_composite_pk);
        let column_name = parsed
            .name
            .clone()
            .unwrap_or_else(|| ident.to_string().to_snake_case());
        let sql_definition = build_sql_definition(SqlDefinition {
            name: &column_name,
            ty: &column_type,
            args: &parsed.type_args,
            not_null: !is_nullable || parsed.not_null,
            constraint: &constraint,
            auto_increment: parsed.is_auto_increment,
            generated: parsed.generated.as_ref(),
            default: parsed.default.as_ref(),
            check: parsed.check.as_deref(),
            charset: parsed.charset.as_deref(),
            collate: parsed.collate.as_deref(),
            on_update: parsed.on_update.as_deref(),
            comment: parsed.comment.as_deref(),
            dynamic_enum: parsed.is_enum,
        });
        let has_default = parsed.default.is_some()
            || parsed.default_fn.is_some()
            || parsed.is_auto_increment
            || parsed.generated.is_some();

        Ok(Self {
            ident,
            field_type,
            base_type,
            column_name,
            column_type,
            type_args: parsed.type_args,
            sql_definition,
            is_nullable,
            is_enum: parsed.is_enum,
            is_set: parsed.is_set,
            is_auto_increment: parsed.is_auto_increment,
            generated_column: parsed.generated,
            default: parsed.default,
            default_fn: parsed.default_fn,
            check_constraint: parsed.check,
            foreign_key: parsed.reference,
            relation_name: parsed.relation_name,
            has_default,
            marker_exprs: parsed.marker_exprs,
            constraint,
            charset: parsed.charset,
            collate: parsed.collate,
            on_update: parsed.on_update,
            comment: parsed.comment,
        })
    }

    #[inline]
    pub fn is_primary(&self) -> bool {
        self.constraint.is_primary()
    }

    #[inline]
    pub fn is_unique(&self) -> bool {
        self.constraint.is_inline_unique()
    }

    pub fn type_category(&self) -> TypeCategory {
        if self.is_enum {
            TypeCategory::Enum
        } else if self.is_set {
            TypeCategory::Set
        } else {
            match mysql_rust_category(&self.base_type) {
                MySQLRustTypeCategory::String => TypeCategory::String,
                MySQLRustTypeCategory::Blob => TypeCategory::Blob,
                _ => TypeCategory::Other,
            }
        }
    }

    pub fn sql_type_expr(&self) -> TokenStream {
        if self.is_enum {
            let ty = &self.base_type;
            quote!(<#ty as drizzle::mysql::traits::MySQLEnum>::SQL_TYPE)
        } else {
            let rendered = render_type(&self.column_type, &self.type_args);
            quote!(#rendered)
        }
    }

    pub fn sql_type_marker(&self) -> TokenStream {
        use MySQLType as T;
        match &self.column_type {
            T::Tinyint => quote!(drizzle::mysql::types::TinyInt),
            T::TinyintUnsigned => quote!(drizzle::mysql::types::TinyIntUnsigned),
            T::Smallint => quote!(drizzle::mysql::types::SmallInt),
            T::SmallintUnsigned => quote!(drizzle::mysql::types::SmallIntUnsigned),
            T::Mediumint => quote!(drizzle::mysql::types::MediumInt),
            T::MediumintUnsigned => quote!(drizzle::mysql::types::MediumIntUnsigned),
            T::Int => quote!(drizzle::mysql::types::Int),
            T::IntUnsigned => quote!(drizzle::mysql::types::IntUnsigned),
            T::Bigint => quote!(drizzle::mysql::types::BigInt),
            T::BigintUnsigned => quote!(drizzle::mysql::types::BigIntUnsigned),
            T::Decimal => quote!(drizzle::mysql::types::Decimal),
            T::Float => quote!(drizzle::mysql::types::Float),
            T::Double => quote!(drizzle::mysql::types::Double),
            T::Boolean => quote!(drizzle::mysql::types::Boolean),
            T::Bit => quote!(drizzle::mysql::types::Bit),
            T::Char => quote!(drizzle::mysql::types::Char),
            T::Varchar => quote!(drizzle::mysql::types::Varchar),
            T::Tinytext => quote!(drizzle::mysql::types::TinyText),
            T::Text => quote!(drizzle::mysql::types::Text),
            T::Mediumtext => quote!(drizzle::mysql::types::MediumText),
            T::Longtext => quote!(drizzle::mysql::types::LongText),
            T::Binary => quote!(drizzle::mysql::types::Binary),
            T::Varbinary => quote!(drizzle::mysql::types::Varbinary),
            T::Tinyblob => quote!(drizzle::mysql::types::TinyBlob),
            T::Blob => quote!(drizzle::mysql::types::Blob),
            T::Mediumblob => quote!(drizzle::mysql::types::MediumBlob),
            T::Longblob => quote!(drizzle::mysql::types::LongBlob),
            T::Json => quote!(drizzle::mysql::types::Json),
            T::Date => quote!(drizzle::mysql::types::Date),
            T::Time => quote!(drizzle::mysql::types::Time),
            T::Datetime => quote!(drizzle::mysql::types::DateTime),
            T::Timestamp => quote!(drizzle::mysql::types::Timestamp),
            T::Year => quote!(drizzle::mysql::types::Year),
            T::Enum(_) => quote!(drizzle::mysql::types::Enum),
            T::Set(_) => quote!(drizzle::mysql::types::Set),
        }
    }

    pub fn sql_definition_expr(&self) -> TokenStream {
        if !self.is_enum {
            let definition = &self.sql_definition;
            return quote!(#definition);
        }
        let const_format = crate::common::paths::const_format();
        let enum_type = self.sql_type_expr();
        let placeholder = "__DRIZZLE_MYSQL_INLINE_ENUM__";
        let (prefix, suffix) = self
            .sql_definition
            .split_once(placeholder)
            .unwrap_or((&self.sql_definition, ""));
        quote!(#const_format::concatcp!(#prefix, #enum_type, #suffix))
    }

    pub fn is_indexable_without_prefix(&self) -> bool {
        !matches!(
            self.column_type,
            MySQLType::Tinytext
                | MySQLType::Text
                | MySQLType::Mediumtext
                | MySQLType::Longtext
                | MySQLType::Tinyblob
                | MySQLType::Blob
                | MySQLType::Mediumblob
                | MySQLType::Longblob
                | MySQLType::Json
        )
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self.column_type,
            MySQLType::Tinyint
                | MySQLType::TinyintUnsigned
                | MySQLType::Smallint
                | MySQLType::SmallintUnsigned
                | MySQLType::Mediumint
                | MySQLType::MediumintUnsigned
                | MySQLType::Int
                | MySQLType::IntUnsigned
                | MySQLType::Bigint
                | MySQLType::BigintUnsigned
                | MySQLType::Decimal
                | MySQLType::Float
                | MySQLType::Double
                | MySQLType::Year
        )
    }

    pub fn direct_index_error(&self) -> &'static str {
        if matches!(self.column_type, MySQLType::Json) {
            "MySQL JSON columns cannot be indexed directly; index a generated scalar column instead"
        } else {
            "MySQL TEXT/BLOB key columns require a prefix length; use a bounded VARCHAR/VARBINARY type until prefix keys are supported"
        }
    }
}

fn parse_column_attrs(field: &Field) -> Result<ParsedColumn> {
    let mut out = ParsedColumn::default();
    for attr in &field.attrs {
        if !attr.path().is_ident("column") {
            continue;
        }
        let metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in metas {
            parse_column_meta(field, meta, &mut out)?;
        }
    }
    if out.reference_on_delete.is_some() || out.reference_on_update.is_some() {
        let Some(reference) = out.reference.as_mut() else {
            return Err(Error::new_spanned(
                field,
                references_required_message(
                    out.reference_on_delete.is_some(),
                    out.reference_on_update.is_some(),
                ),
            ));
        };
        reference.on_delete = out.reference_on_delete.take();
        reference.on_update = out.reference_on_update.take();
    }
    Ok(out)
}

fn parse_column_meta(field: &Field, meta: Meta, out: &mut ParsedColumn) -> Result<()> {
    let name = meta
        .path()
        .get_ident()
        .map(ToString::to_string)
        .ok_or_else(|| Error::new_spanned(&meta, "expected a MySQL column attribute name"))?;
    let upper = name.to_ascii_uppercase();
    match meta {
        Meta::Path(path) => match upper.as_str() {
            "PRIMARY" | "PRIMARY_KEY" => {
                out.primary = true;
                out.marker_exprs.push(make_uppercase_path(
                    path.get_ident().expect("checked ident"),
                    "PRIMARY",
                ));
            }
            "UNIQUE" => {
                out.unique = true;
                out.marker_exprs.push(make_uppercase_path(
                    path.get_ident().expect("checked ident"),
                    "UNIQUE",
                ));
            }
            "NOT_NULL" => out.not_null = true,
            "AUTO_INCREMENT" => out.is_auto_increment = true,
            "AUTOINCREMENT" => {
                return Err(Error::new_spanned(
                    path,
                    "AUTOINCREMENT is SQLite-only; MySQL uses AUTO_INCREMENT",
                ));
            }
            "ENUM" => out.is_enum = true,
            "JSON" => set_explicit_type(field, out, MySQLType::Json)?,
            "JSONB" => {
                return Err(Error::new_spanned(
                    path,
                    "JSONB is PostgreSQL/SQLite-only; MySQL uses JSON",
                ));
            }
            "SERIAL" | "BIGSERIAL" | "SMALLSERIAL" | "IDENTITY" | "PGENUM" => {
                return Err(Error::new_spanned(
                    path,
                    format!(
                        "{upper} is PostgreSQL-only; use a MySQL integer with AUTO_INCREMENT or inline ENUM"
                    ),
                ));
            }
            "DEFERRABLE" | "INITIALLY_DEFERRED" => {
                return Err(Error::new_spanned(
                    path,
                    "MySQL foreign keys are not deferrable",
                ));
            }
            _ => {
                let ty = MySQLType::parse_attribute(&upper).ok_or_else(|| {
                    Error::new_spanned(
                        path,
                        format!("unrecognized MySQL column attribute `{name}`"),
                    )
                })?;
                set_explicit_type(field, out, ty)?;
            }
        },
        Meta::NameValue(value) => match upper.as_str() {
            "NAME" => out.name = Some(expect_string(&value.value, "NAME")?),
            "DEFAULT" => out.default = Some(default_from_expr(&value.value)?),
            "DEFAULT_SQL" => {
                out.default = Some(MySQLDefault::RawSql(expect_string(
                    &value.value,
                    "DEFAULT_SQL",
                )?));
            }
            "DEFAULT_FN" => out.default_fn = Some(value.value.to_token_stream()),
            "CHECK" => out.check = Some(expect_string(&value.value, "CHECK")?),
            "REFERENCES" => out.reference = Some(parse_reference_expr(&value.value)?),
            "RELATION" => {
                let name = expect_string(&value.value, "RELATION")?;
                if syn::parse_str::<Ident>(&name).is_err() {
                    return Err(Error::new_spanned(
                        value,
                        format!("RELATION = \"{name}\" must be a valid Rust identifier"),
                    ));
                }
                out.relation_name = Some(name);
            }
            "ON_DELETE" => {
                out.reference_on_delete = Some(parse_referential_action(&value.value)?);
            }
            "ON_UPDATE" => {
                if matches!(&value.value, Expr::Path(_)) {
                    out.reference_on_update = Some(parse_referential_action(&value.value)?);
                } else {
                    out.on_update = Some(expect_string(&value.value, "ON_UPDATE")?);
                }
            }
            "CHARSET" | "CHARACTER_SET" => {
                out.charset = Some(expect_string(&value.value, "CHARACTER_SET")?);
            }
            "COLLATE" => out.collate = Some(expect_string(&value.value, "COLLATE")?),
            "COMMENT" => out.comment = Some(expect_string(&value.value, "COMMENT")?),
            _ => {
                return Err(Error::new_spanned(
                    value,
                    format!("unrecognized MySQL column attribute `{name}`"),
                ));
            }
        },
        Meta::List(list) => match upper.as_str() {
            "GENERATED" => {
                let args = syn::parse2::<GeneratedArgs>(list.tokens)?;
                out.generated = Some(GeneratedColumn {
                    expression: args.expression,
                    stored: args.stored,
                });
            }
            "SET" => {
                let values = Punctuated::<syn::LitStr, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())
                    .map_err(|_| {
                        Error::new_spanned(&list, "SET expects string values: SET(\"a\", \"b\")")
                    })?;
                if values.is_empty() {
                    return Err(Error::new_spanned(
                        &list,
                        "MySQL SET requires at least one value",
                    ));
                }
                if values.len() > 64 {
                    return Err(Error::new_spanned(
                        &list,
                        "MySQL SET supports at most 64 values",
                    ));
                }
                let values = values
                    .into_iter()
                    .map(|value| validate_inline_label(&value).map(|()| value.value()))
                    .collect::<Result<Vec<_>>>()?;
                out.is_set = true;
                set_explicit_type(field, out, MySQLType::set_values(values))?;
            }
            _ => {
                let ty = MySQLType::parse_attribute(&upper).ok_or_else(|| {
                    Error::new_spanned(
                        &list,
                        format!("unrecognized MySQL column attribute `{name}`"),
                    )
                })?;
                out.type_args = parse_type_args(&list)?;
                set_explicit_type(field, out, ty)?;
            }
        },
    }
    Ok(())
}

fn set_explicit_type(field: &Field, out: &mut ParsedColumn, ty: MySQLType) -> Result<()> {
    if out.explicit_type.replace(ty).is_some() {
        return Err(Error::new_spanned(
            field,
            "a MySQL column may specify only one SQL type",
        ));
    }
    Ok(())
}

fn parse_type_args(list: &syn::MetaList) -> Result<Vec<u16>> {
    let args = list.parse_args_with(Punctuated::<syn::LitInt, Token![,]>::parse_terminated)?;
    args.into_iter()
        .map(|value| value.base10_parse::<u16>())
        .collect()
}

fn validate_type_args(field: &Field, ty: &MySQLType, args: &[u16]) -> Result<()> {
    let bad = |message| Error::new_spanned(field, message);
    match ty {
        MySQLType::Varchar | MySQLType::Varbinary if args.len() != 1 => Err(bad(
            "VARCHAR and VARBINARY require exactly one length argument",
        )),
        MySQLType::Char | MySQLType::Binary if args.len() > 1 => {
            Err(bad("CHAR and BINARY accept at most one length argument"))
        }
        MySQLType::Bit if args.len() > 1 => Err(bad("BIT accepts at most one width argument")),
        MySQLType::Decimal if args.len() > 2 => {
            Err(bad("DECIMAL accepts precision and optional scale"))
        }
        MySQLType::Time | MySQLType::Datetime | MySQLType::Timestamp if args.len() > 1 => Err(bad(
            "temporal types accept at most one fractional-seconds precision",
        )),
        _ if !args.is_empty()
            && !matches!(
                ty,
                MySQLType::Varchar
                    | MySQLType::Varbinary
                    | MySQLType::Char
                    | MySQLType::Binary
                    | MySQLType::Bit
                    | MySQLType::Decimal
                    | MySQLType::Time
                    | MySQLType::Datetime
                    | MySQLType::Timestamp
            ) =>
        {
            Err(bad(
                "this MySQL type does not accept length, precision, or scale arguments",
            ))
        }
        MySQLType::Bit if args.first().is_some_and(|value| !(1..=64).contains(value)) => {
            Err(bad("BIT width must be between 1 and 64"))
        }
        MySQLType::Char | MySQLType::Binary if args.first().is_some_and(|value| *value > 255) => {
            Err(bad("CHAR/BINARY length must not exceed 255"))
        }
        MySQLType::Decimal if args.first().is_some_and(|value| !(1..=65).contains(value)) => {
            Err(bad("DECIMAL precision must be between 1 and 65"))
        }
        MySQLType::Decimal
            if args.get(1).is_some_and(|scale| {
                *scale > 30 || args.first().is_some_and(|precision| scale > precision)
            }) =>
        {
            Err(bad("DECIMAL scale must not exceed 30 or its precision"))
        }
        MySQLType::Time | MySQLType::Datetime | MySQLType::Timestamp
            if args.first().is_some_and(|value| *value > 6) =>
        {
            Err(bad("fractional-seconds precision must be between 0 and 6"))
        }
        _ => Ok(()),
    }
}

fn mysql_rust_category(ty: &Type) -> MySQLRustTypeCategory {
    if type_is_array_u8(ty) {
        MySQLRustTypeCategory::ByteArray
    } else if type_is_array_char(ty) {
        MySQLRustTypeCategory::CharArray
    } else if type_is_array_string(ty) {
        MySQLRustTypeCategory::ArrayString
    } else if type_is_arrayvec_u8(ty) {
        MySQLRustTypeCategory::ArrayVec
    } else if type_is_uuid(ty) {
        MySQLRustTypeCategory::Uuid
    } else if type_is_json_value(ty) {
        MySQLRustTypeCategory::Json
    } else if type_is_naive_datetime(ty) {
        MySQLRustTypeCategory::NaiveDateTime
    } else if type_is_naive_date(ty) {
        MySQLRustTypeCategory::NaiveDate
    } else if type_is_naive_time(ty) {
        MySQLRustTypeCategory::NaiveTime
    } else if type_is_datetime_tz(ty) {
        MySQLRustTypeCategory::DateTimeTz
    } else if type_is_primitive_date_time(ty) {
        MySQLRustTypeCategory::TimePrimitiveDateTime
    } else if type_is_offset_datetime(ty) {
        MySQLRustTypeCategory::TimeOffsetDateTime
    } else if type_is_time_date(ty) {
        MySQLRustTypeCategory::TimeDate
    } else if type_is_time_time(ty) {
        MySQLRustTypeCategory::TimeTime
    } else if type_is_string_like(ty) {
        MySQLRustTypeCategory::String
    } else if type_is_vec_u8(ty) {
        MySQLRustTypeCategory::Blob
    } else if type_is_bool(ty) {
        MySQLRustTypeCategory::Bool
    } else if type_is_float(ty, "f32") {
        MySQLRustTypeCategory::F32
    } else if type_is_float(ty, "f64") {
        MySQLRustTypeCategory::F64
    } else if type_is_int(ty, "i8") {
        MySQLRustTypeCategory::I8
    } else if type_is_int(ty, "i16") {
        MySQLRustTypeCategory::I16
    } else if type_is_int(ty, "i32") {
        MySQLRustTypeCategory::I32
    } else if type_is_int(ty, "i64") {
        MySQLRustTypeCategory::I64
    } else if type_is_int(ty, "isize") {
        MySQLRustTypeCategory::Isize
    } else if type_is_int(ty, "u8") {
        MySQLRustTypeCategory::U8
    } else if type_is_int(ty, "u16") {
        MySQLRustTypeCategory::U16
    } else if type_is_int(ty, "u32") {
        MySQLRustTypeCategory::U32
    } else if type_is_int(ty, "u64") {
        MySQLRustTypeCategory::U64
    } else if type_is_int(ty, "usize") {
        MySQLRustTypeCategory::Usize
    } else {
        MySQLRustTypeCategory::Unknown
    }
}

fn infer_mysql_type(
    field: &Field,
    ty: &Type,
    category: MySQLRustTypeCategory,
) -> Result<(MySQLType, Vec<u16>)> {
    let inferred = match category {
        #[cfg(feature = "uuid")]
        MySQLRustTypeCategory::Uuid => Some((MySQLType::Binary, vec![16])),
        MySQLRustTypeCategory::ArrayString
            if last_type_ident(ty).is_some_and(|ident| ident == "CompactString") =>
        {
            Some((MySQLType::Text, Vec::new()))
        }
        MySQLRustTypeCategory::ArrayString => Some((
            MySQLType::Varchar,
            vec![const_generic_capacity(field, ty, "ArrayString")?],
        )),
        MySQLRustTypeCategory::ByteArray => Some((
            MySQLType::Binary,
            vec![array_capacity(field, ty, "byte array")?],
        )),
        MySQLRustTypeCategory::CharArray => Some((
            MySQLType::Char,
            vec![array_capacity(field, ty, "character array")?],
        )),
        MySQLRustTypeCategory::ArrayVec
            if last_type_ident(ty).is_some_and(|ident| ident == "ArrayVec") =>
        {
            Some((
                MySQLType::Varbinary,
                vec![const_generic_capacity(field, ty, "ArrayVec")?],
            ))
        }
        MySQLRustTypeCategory::ArrayVec => Some((MySQLType::Blob, Vec::new())),
        _ => None,
    };

    inferred.or_else(|| category.sql_type().map(|ty| (ty, Vec::new()))).ok_or_else(|| {
        Error::new_spanned(
            &field.ty,
            format!(
                "unsupported Rust type `{}` for MySQL; add #[derive(MySQLEnum)] and #[column(ENUM)], or specify a supported MySQL type",
                field.ty.to_token_stream()
            ),
        )
    })
}

fn last_type_ident(ty: &Type) -> Option<&Ident> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|segment| &segment.ident)
}

fn array_capacity(field: &Field, ty: &Type, kind: &str) -> Result<u16> {
    let Type::Array(array) = ty else {
        return Err(Error::new_spanned(
            field,
            format!("MySQL {kind} inference requires an array type"),
        ));
    };
    literal_capacity(&array.len, kind)
}

fn const_generic_capacity(field: &Field, ty: &Type, kind: &str) -> Result<u16> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(
            field,
            format!("MySQL {kind} inference requires a literal capacity"),
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new_spanned(
            field,
            format!("MySQL {kind} inference requires a literal capacity"),
        ));
    };
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(Error::new_spanned(
            field,
            format!("MySQL {kind} inference requires a literal capacity"),
        ));
    };
    let Some(expression) = arguments.args.iter().rev().find_map(|argument| {
        if let GenericArgument::Const(expression) = argument {
            Some(expression)
        } else {
            None
        }
    }) else {
        return Err(Error::new_spanned(
            field,
            format!("MySQL {kind} inference requires a literal capacity"),
        ));
    };
    literal_capacity(expression, kind)
}

fn literal_capacity(expression: &Expr, kind: &str) -> Result<u16> {
    let Expr::Lit(expression) = expression else {
        return Err(Error::new_spanned(
            expression,
            format!("MySQL {kind} capacity must be an integer literal"),
        ));
    };
    let Lit::Int(value) = &expression.lit else {
        return Err(Error::new_spanned(
            expression,
            format!("MySQL {kind} capacity must be an integer literal"),
        ));
    };
    value.base10_parse::<u16>().map_err(|_| {
        Error::new_spanned(
            value,
            format!("MySQL {kind} capacity must fit in an unsigned 16-bit length"),
        )
    })
}

fn validate_explicit_signedness(
    field: &Field,
    rust: MySQLRustTypeCategory,
    sql: &MySQLType,
) -> Result<()> {
    let rust_unsigned = matches!(
        rust,
        MySQLRustTypeCategory::U8
            | MySQLRustTypeCategory::U16
            | MySQLRustTypeCategory::U32
            | MySQLRustTypeCategory::U64
            | MySQLRustTypeCategory::Usize
    );
    let rust_signed = matches!(
        rust,
        MySQLRustTypeCategory::I8
            | MySQLRustTypeCategory::I16
            | MySQLRustTypeCategory::I32
            | MySQLRustTypeCategory::I64
            | MySQLRustTypeCategory::Isize
    );
    if sql.is_unsigned() && rust_signed {
        return Err(Error::new_spanned(
            field,
            format!(
                "{} is unsigned but the Rust field is signed; use the corresponding u8/u16/u32/u64 type",
                sql.sql()
            ),
        ));
    }
    if is_signed_integer(sql) && rust_unsigned {
        return Err(Error::new_spanned(
            field,
            format!(
                "{} is signed but the Rust field is unsigned; use the corresponding signed Rust type or an *_UNSIGNED SQL type",
                sql.sql()
            ),
        ));
    }
    Ok(())
}

fn is_signed_integer(ty: &MySQLType) -> bool {
    matches!(
        ty,
        MySQLType::Tinyint
            | MySQLType::Smallint
            | MySQLType::Mediumint
            | MySQLType::Int
            | MySQLType::Bigint
    )
}

fn supports_character_options(ty: &MySQLType) -> bool {
    matches!(
        ty,
        MySQLType::Char
            | MySQLType::Varchar
            | MySQLType::Tinytext
            | MySQLType::Text
            | MySQLType::Mediumtext
            | MySQLType::Longtext
            | MySQLType::Enum(_)
            | MySQLType::Set(_)
    )
}

fn requires_expression_default(ty: &MySQLType) -> bool {
    matches!(
        ty,
        MySQLType::Tinytext
            | MySQLType::Text
            | MySQLType::Mediumtext
            | MySQLType::Longtext
            | MySQLType::Tinyblob
            | MySQLType::Blob
            | MySQLType::Mediumblob
            | MySQLType::Longblob
            | MySQLType::Json
    )
}

struct SqlDefinition<'a> {
    name: &'a str,
    ty: &'a MySQLType,
    args: &'a [u16],
    not_null: bool,
    constraint: &'a Constraint,
    auto_increment: bool,
    generated: Option<&'a GeneratedColumn>,
    default: Option<&'a MySQLDefault>,
    check: Option<&'a str>,
    charset: Option<&'a str>,
    collate: Option<&'a str>,
    on_update: Option<&'a str>,
    comment: Option<&'a str>,
    dynamic_enum: bool,
}

fn build_sql_definition(definition: SqlDefinition<'_>) -> String {
    let SqlDefinition {
        name,
        ty,
        args,
        not_null,
        constraint,
        auto_increment,
        generated,
        default,
        check,
        charset,
        collate,
        on_update,
        comment,
        dynamic_enum,
    } = definition;
    let mut sql = format!(
        "`{}` {}",
        name.replace('`', "``"),
        if dynamic_enum {
            "__DRIZZLE_MYSQL_INLINE_ENUM__".to_string()
        } else {
            render_type(ty, args)
        }
    );
    if let Some(charset) = charset {
        let _ = write!(sql, " CHARACTER SET `{}`", charset.replace('`', "``"));
    }
    if let Some(collate) = collate {
        let _ = write!(sql, " COLLATE `{}`", collate.replace('`', "``"));
    }
    if let Some(generated) = generated {
        let kind = if generated.stored {
            "STORED"
        } else {
            "VIRTUAL"
        };
        let _ = write!(
            sql,
            " GENERATED ALWAYS AS ({}) {kind}",
            generated.expression
        );
    }
    if constraint.is_inline_primary() {
        sql.push_str(" PRIMARY KEY");
    } else if constraint.is_inline_unique() {
        sql.push_str(" UNIQUE");
    }
    if not_null {
        sql.push_str(" NOT NULL");
    }
    if auto_increment {
        sql.push_str(" AUTO_INCREMENT");
    }
    if generated.is_none()
        && let Some(default) = default
    {
        match default {
            MySQLDefault::Literal(value) | MySQLDefault::RawSql(value) => {
                if requires_expression_default(ty) {
                    let _ = write!(sql, " DEFAULT ({value})");
                } else {
                    let _ = write!(sql, " DEFAULT {value}");
                }
            }
        }
    }
    if let Some(on_update) = on_update {
        let _ = write!(sql, " ON UPDATE {on_update}");
    }
    if let Some(check) = check {
        let _ = write!(sql, " CHECK ({check})");
    }
    if let Some(comment) = comment {
        let _ = write!(sql, " COMMENT '{}'", escape_mysql_string(comment));
    }
    sql
}

fn render_type(ty: &MySQLType, args: &[u16]) -> String {
    match ty {
        MySQLType::Enum(_) => "ENUM".to_string(),
        MySQLType::Set(values) => format!(
            "SET({})",
            values
                .iter()
                .map(|value| format!("'{}'", value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ if args.is_empty() => ty.sql().to_string(),
        _ => format!(
            "{}({})",
            ty.sql(),
            args.iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn default_from_expr(expr: &Expr) -> Result<MySQLDefault> {
    let Expr::Lit(lit) = expr else {
        return Err(Error::new_spanned(
            expr,
            "DEFAULT requires a string, number, boolean, or character literal; use DEFAULT_SQL for SQL expressions",
        ));
    };
    let rendered = match &lit.lit {
        Lit::Str(value) => format!("'{}'", escape_mysql_string(&value.value())),
        Lit::ByteStr(value) => format!("X'{}'", hex(value.value())),
        Lit::Byte(value) => value.value().to_string(),
        Lit::Char(value) => format!("'{}'", escape_mysql_string(&value.value().to_string())),
        Lit::Int(value) => value.base10_digits().to_string(),
        Lit::Float(value) => value.base10_digits().to_string(),
        Lit::Bool(value) => (if value.value { "TRUE" } else { "FALSE" }).to_string(),
        _ => {
            return Err(Error::new_spanned(
                expr,
                "unsupported MySQL DEFAULT literal",
            ));
        }
    };
    Ok(MySQLDefault::Literal(rendered))
}

fn expect_string(expr: &Expr, attribute: &str) -> Result<String> {
    if let Expr::Lit(syn::ExprLit {
        lit: Lit::Str(value),
        ..
    }) = expr
    {
        Ok(value.value())
    } else {
        Err(Error::new_spanned(
            expr,
            format!("{attribute} requires a string literal"),
        ))
    }
}

fn parse_reference_expr(expr: &Expr) -> Result<MySQLReference> {
    let Expr::Path(path) = expr else {
        return Err(Error::new_spanned(
            expr,
            references_required_message(false, true),
        ));
    };
    if path.path.segments.len() != 2 {
        return Err(Error::new_spanned(
            expr,
            references_required_message(false, true),
        ));
    }
    let table = path
        .path
        .segments
        .first()
        .expect("length checked")
        .ident
        .clone();
    let column = path
        .path
        .segments
        .last()
        .expect("length checked")
        .ident
        .clone();
    Ok(MySQLReference {
        table,
        column,
        on_delete: None,
        on_update: None,
    })
}

fn parse_referential_action(expr: &Expr) -> Result<String> {
    let Expr::Path(path) = expr else {
        return Err(Error::new_spanned(
            expr,
            "referential actions use CASCADE, SET_NULL, RESTRICT, or NO_ACTION",
        ));
    };
    let ident = path
        .path
        .get_ident()
        .ok_or_else(|| Error::new_spanned(expr, "expected a referential action"))?;
    match ident.to_string().to_ascii_uppercase().as_str() {
        "CASCADE" => Ok("CASCADE".to_string()),
        "SET_NULL" => Ok("SET NULL".to_string()),
        "RESTRICT" => Ok("RESTRICT".to_string()),
        "NO_ACTION" => Ok("NO ACTION".to_string()),
        "SET_DEFAULT" => Err(Error::new_spanned(
            expr,
            "InnoDB rejects SET DEFAULT referential actions",
        )),
        _ => Err(Error::new_spanned(
            expr,
            "invalid MySQL referential action; expected CASCADE, SET_NULL, RESTRICT, or NO_ACTION",
        )),
    }
}

fn validate_inline_label(value: &syn::LitStr) -> Result<()> {
    if value.value().contains(['\'', '\\']) {
        Err(Error::new_spanned(
            value,
            "inline SET values containing quotes or backslashes are not supported because their meaning depends on MySQL SQL mode",
        ))
    } else {
        Ok(())
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

fn parse_quote_path(name: &str) -> ExprPath {
    syn::parse_str(name).expect("static marker path")
}

impl crate::common::constraints::ForeignKeyRef for MySQLReference {
    fn ref_table(&self) -> &Ident {
        &self.table
    }

    fn ref_column(&self) -> &Ident {
        &self.column
    }
}

impl crate::common::constraints::ConstraintFieldInfo for FieldInfo {
    type ForeignKey = MySQLReference;

    fn ident(&self) -> &Ident {
        &self.ident
    }

    fn column_name(&self) -> &str {
        &self.column_name
    }

    fn is_primary(&self) -> bool {
        self.is_primary()
    }

    fn is_unique(&self) -> bool {
        self.is_unique()
    }

    fn foreign_key(&self) -> Option<&Self::ForeignKey> {
        self.foreign_key.as_ref()
    }
}

pub fn generate_table_meta_json(table_name: &str, fields: &[FieldInfo]) -> String {
    let columns = fields
        .iter()
        .map(|field| {
            serde_json::json!({
                "name": field.column_name,
                "type": if field.is_enum { "ENUM".to_string() } else { render_type(&field.column_type, &field.type_args) },
                "notNull": !field.is_nullable,
                "primaryKey": field.is_primary(),
                "autoIncrement": field.is_auto_increment,
                "generated": field.generated_column.as_ref().map(|generated| serde_json::json!({
                    "expression": generated.expression,
                    "stored": generated.stored,
                })),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "dialect": "mysql",
        "table": table_name,
        "columns": columns,
    })
    .to_string()
}
