//! Const DDL generation for `SQLite` tables.
//!
//! This module emits the `CREATE TABLE` SQL stored on each table's
//! `SQLSchema::SQL` const. The output is byte-for-byte identical to the
//! runtime `TableSql::create_table_sql()` ([`drizzle_types::sqlite::ddl::sql`])
//! so that compile-time and runtime DDL never diverge.
//!
//! Because the compile-time emitter has to splice in `<OtherTable>::TABLE_NAME`
//! const refs for foreign-key targets (the referenced table's name isn't
//! known to *this* macro), every const is built as `concatcp!(...)` of
//! [`DdlPiece`]s. When there are no FKs, the concatcp! degenerates into a
//! single literal at compile time — no runtime cost.

use super::context::MacroContext;
use crate::paths::ddl::sqlite as ddl_paths;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use crate::sqlite::field::FieldInfo;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Write;

/// A single piece of a `concatcp!`-emitted CREATE TABLE statement.
///
/// Most pieces are plain literals (`Self::Literal`). When a foreign-key clause
/// needs to reference another table's name, we emit a `Self::TableNameOf` that
/// expands to `<RefTable>::TABLE_NAME` so the lookup happens at compile time.
enum DdlPiece {
    Literal(String),
    TableNameOf(syn::Ident),
}

impl DdlPiece {
    fn to_token(&self) -> TokenStream {
        match self {
            Self::Literal(s) => quote! { #s },
            Self::TableNameOf(ident) => quote! { <#ident>::TABLE_NAME },
        }
    }
}

/// Convert a referential action string to the corresponding enum variant token
fn referential_action_token(action: &str, referential_action: &TokenStream) -> TokenStream {
    match action.to_uppercase().as_str() {
        "RESTRICT" => quote! { #referential_action::Restrict },
        "CASCADE" => quote! { #referential_action::Cascade },
        "SET NULL" => quote! { #referential_action::SetNull },
        "SET DEFAULT" => quote! { #referential_action::SetDefault },
        // "NO ACTION" and unknown values default to NoAction
        _ => quote! { #referential_action::NoAction },
    }
}

/// Generate a compile-time `const SQL: &'static str` value for `SQLSchema`.
///
/// Output matches `TableSql::create_table_sql()` exactly: backtick-quoted
/// identifiers, named `CONSTRAINT` clauses for primary/foreign/unique
/// constraints, `ON UPDATE` before `ON DELETE`, `NO ACTION` skipped, and
/// trailing `;`.
pub fn generate_schema_sql_const(ctx: &MacroContext) -> TokenStream {
    let tokens: Vec<TokenStream> = build_create_table_pieces(ctx)
        .iter()
        .map(DdlPiece::to_token)
        .collect();
    let const_format = crate::common::paths::const_format();
    quote! {
        #const_format::concatcp!(#(#tokens),*)
    }
}

/// Build the full CREATE TABLE statement as an ordered list of `DdlPiece`s.
///
/// The pieces are designed to be passed straight to `concatcp!`. Lines are
/// joined by literal `",\n"` prefixes on every non-first line.
fn build_create_table_pieces(ctx: &MacroContext) -> Vec<DdlPiece> {
    let table_name = &ctx.table_name;
    let is_composite_pk = ctx.is_composite_pk;
    let strict = ctx.attrs.strict;
    let without_rowid = ctx.attrs.without_rowid;
    let field_infos = ctx.field_infos;

    let mut pieces: Vec<DdlPiece> = Vec::new();
    pieces.push(DdlPiece::Literal(format!(
        "CREATE TABLE `{table_name}` (\n"
    )));

    // Lines are built as `Vec<DdlPiece>` so the FK case can splice in a
    // `<RefTable>::TABLE_NAME` piece between literal fragments.
    let mut lines: Vec<Vec<DdlPiece>> = Vec::new();

    // Column definitions. The two "is this column the inline PK / inline
    // UNIQUE?" questions are answered by `Constraint`, set during the
    // table-level pass.
    for field in field_infos {
        lines.push(vec![DdlPiece::Literal(format!(
            "\t{}",
            column_to_sql(
                field,
                field.constraint.is_inline_primary(),
                field.constraint.is_inline_unique(),
            )
        ))]);
    }

    // Composite primary key (only when there are 2+ PK columns)
    if is_composite_pk {
        let pk_cols: Vec<String> = field_infos
            .iter()
            .filter(|f| f.is_primary())
            .map(|f| format!("`{}`", f.column_name))
            .collect();
        if !pk_cols.is_empty() {
            let pk_name = format!("{table_name}_pkey");
            lines.push(vec![DdlPiece::Literal(format!(
                "\tCONSTRAINT `{}` PRIMARY KEY({})",
                pk_name,
                pk_cols.join(", ")
            ))]);
        }
    }

    // Single-column foreign keys (CONSTRAINT name + FOREIGN KEY ... REFERENCES)
    for field in field_infos {
        if let Some(ref fk) = field.foreign_key {
            let fk_name = format!("{}_{}_fkey", table_name, field.column_name);
            let ref_column = fk.column_ident.to_string().to_snake_case();
            let mut line = Vec::new();
            line.push(DdlPiece::Literal(format!(
                "\tCONSTRAINT `{}` FOREIGN KEY (`{}`) REFERENCES `",
                fk_name, field.column_name
            )));
            line.push(DdlPiece::TableNameOf(fk.table_ident.clone()));
            let mut suffix = format!("`(`{ref_column}`)");
            // Match TableSql::to_constraint_sql ordering: ON UPDATE then ON DELETE,
            // and skip the no-op NO ACTION.
            if let Some(ref on_update) = fk.on_update {
                let action = on_update.to_uppercase();
                if action != "NO ACTION" {
                    let _ = write!(suffix, " ON UPDATE {action}");
                }
            }
            if let Some(ref on_delete) = fk.on_delete {
                let action = on_delete.to_uppercase();
                if action != "NO ACTION" {
                    let _ = write!(suffix, " ON DELETE {action}");
                }
            }
            line.push(DdlPiece::Literal(suffix));
            lines.push(line);
        }
    }

    // Composite foreign keys
    for fk in &ctx.attrs.composite_foreign_keys {
        let source_cols: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let target_cols: Vec<String> = fk
            .target_columns
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let fk_name = format!("{}_{}_fkey", table_name, source_cols.join("_"));
        let src_str = source_cols
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let tgt_str = target_cols
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ");

        let mut line = Vec::new();
        line.push(DdlPiece::Literal(format!(
            "\tCONSTRAINT `{fk_name}` FOREIGN KEY ({src_str}) REFERENCES `"
        )));
        line.push(DdlPiece::TableNameOf(fk.target_table.clone()));
        let mut suffix = format!("`({tgt_str})");
        if let Some(ref on_update) = fk.on_update {
            let action = on_update.to_uppercase();
            if action != "NO ACTION" {
                let _ = write!(suffix, " ON UPDATE {action}");
            }
        }
        if let Some(ref on_delete) = fk.on_delete {
            let action = on_delete.to_uppercase();
            if action != "NO ACTION" {
                let _ = write!(suffix, " ON DELETE {action}");
            }
        }
        line.push(DdlPiece::Literal(suffix));
        lines.push(line);
    }

    // Multi-column unique constraints would go here. Currently the field
    // model only carries single-column UNIQUE which is rendered inline via
    // `column_to_sql`, so there's nothing to emit at the table level.

    // Join lines with ",\n" — prepend it to the first piece of every non-first line.
    for (i, mut line) in lines.into_iter().enumerate() {
        if i == 0 {
            pieces.append(&mut line);
            continue;
        }
        match line.first_mut() {
            Some(DdlPiece::Literal(s)) => *s = format!(",\n{s}"),
            _ => pieces.push(DdlPiece::Literal(",\n".to_string())),
        }
        pieces.append(&mut line);
    }

    // Closing: \n) + options + ;
    let mut closing = "\n)".to_string();
    if without_rowid {
        closing.push_str(" WITHOUT ROWID");
    }
    if strict {
        closing.push_str(" STRICT");
    }
    closing.push(';');
    pieces.push(DdlPiece::Literal(closing));

    pieces
}

/// Format a single column's SQL fragment.
///
/// Mirrors `Column::to_column_sql` in `drizzle_types::sqlite::ddl::sql` so the
/// const SQL stays consistent with the runtime emitter:
/// - identifiers are backtick-quoted,
/// - `PRIMARY KEY` / `AUTOINCREMENT` are inlined only when this column is a
///   single-column primary key,
/// - `NOT NULL` is skipped on `INTEGER PRIMARY KEY` (SQLite quirk: such
///   columns alias `rowid` and accept NULL on insert),
/// - `UNIQUE` is inlined when the column carries a single-column unique that
///   isn't already implied by being the primary key.
fn column_to_sql(field: &FieldInfo, inline_pk: bool, inline_unique: bool) -> String {
    let mut sql = format!(
        "`{}` {}",
        field.column_name,
        field.column_type.to_sql_type()
    );

    if inline_pk {
        sql.push_str(" PRIMARY KEY");
        if field.is_autoincrement {
            sql.push_str(" AUTOINCREMENT");
        }
    }

    if let Some(ref default_expr) = field.default_value
        && let syn::Expr::Lit(expr_lit) = default_expr
    {
        let default_str = match &expr_lit.lit {
            syn::Lit::Int(i) => format!(" DEFAULT {i}"),
            syn::Lit::Float(f) => format!(" DEFAULT {f}"),
            syn::Lit::Bool(b) => format!(" DEFAULT {}", if b.value() { "1" } else { "0" }),
            syn::Lit::Str(s) => format!(" DEFAULT '{}'", s.value().replace('\'', "''")),
            _ => String::new(),
        };
        if !default_str.is_empty() {
            sql.push_str(&default_str);
        }
    }

    let sql_type = field.column_type.to_sql_type();
    if !(field.is_nullable || inline_pk && sql_type.to_lowercase().starts_with("int")) {
        sql.push_str(" NOT NULL");
    }

    if inline_unique && !inline_pk {
        sql.push_str(" UNIQUE");
    }

    // COLLATE follows inline constraints (matches `Column::to_column_sql` in
    // drizzle_types so the const SQL and runtime emitter stay in sync).
    if let Some(ref name) = field.collate {
        let _ = write!(sql, " COLLATE {name}");
    }

    sql
}

/// Generate const DDL definitions for the table and its columns.
///
/// This generates:
/// - `DDL_TABLE: drizzle_types::sqlite::ddl::TableDef` - Table definition
/// - `DDL_COLUMNS: &'static [drizzle_types::sqlite::ddl::ColumnDef]` - Column definitions
/// - `DDL_PRIMARY_KEY: Option<...>` - Primary key definition
/// - `DDL_FOREIGN_KEYS: &'static [...]` - Foreign key definitions
/// - `DDL_UNIQUE_CONSTRAINTS: &'static [...]` - Unique constraint definitions
pub fn generate_const_ddl(ctx: &MacroContext) -> TokenStream {
    let table_name = &ctx.table_name;
    let strict = ctx.attrs.strict;
    let without_rowid = ctx.attrs.without_rowid;

    // Get core type paths for SQLSchema reference
    let sql_schema = core_paths::sql_schema();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_value = sqlite_paths::sqlite_value();

    // Get DDL type paths
    let table_def = ddl_paths::table_def();
    let column_def = ddl_paths::column_def();
    let primary_key_def = ddl_paths::primary_key_def();
    let foreign_key_def = ddl_paths::foreign_key_def();
    let unique_constraint_def = ddl_paths::unique_constraint_def();
    let index_def = ddl_paths::index_def();
    let table_sql = ddl_paths::table_sql();
    let referential_action = ddl_paths::referential_action();

    // Generate table modifiers
    let mut table_modifiers = Vec::new();
    if strict {
        table_modifiers.push(quote! { .strict() });
    }
    if without_rowid {
        table_modifiers.push(quote! { .without_rowid() });
    }

    // Generate column definitions
    let column_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .map(|field| {
            let column_name = &field.column_name;
            let sql_type = field.column_type.to_sql_type();

            let mut modifiers = Vec::new();

            if !field.is_nullable {
                modifiers.push(quote! { .not_null() });
            }
            if field.is_primary() && !field.is_autoincrement {
                modifiers.push(quote! { .primary_key() });
            }
            if field.is_autoincrement {
                modifiers.push(quote! { .autoincrement() });
            }
            if field.is_unique() {
                modifiers.push(quote! { .unique() });
            }
            if let Some(syn::Expr::Lit(expr_lit)) = field.default_value.as_ref() {
                // Convert the expression to a string for DDL
                let default_str = match &expr_lit.lit {
                    syn::Lit::Int(i) => i.to_string(),
                    syn::Lit::Float(f) => f.to_string(),
                    syn::Lit::Bool(b) => if b.value() { "1" } else { "0" }.to_string(),
                    syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "''")),
                    _ => String::new(),
                };
                if !default_str.is_empty() {
                    modifiers.push(quote! { .default_value(#default_str) });
                }
            }
            if let Some(ref collate_name) = field.collate {
                modifiers.push(quote! { .collate(#collate_name) });
            }

            quote! {
                #column_def::new(#table_name, #column_name, #sql_type)
                #(#modifiers)*
            }
        })
        .collect();

    // Build primary key DDL if there are primary key columns
    let pk_columns: Vec<&String> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_primary())
        .map(|f| &f.column_name)
        .collect();

    let pk_name = format!("{table_name}_pkey");
    let pk_def = if pk_columns.is_empty() {
        quote! {
            /// Primary key definition (none)
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> =
                ::std::option::Option::None;
        }
    } else {
        let pk_col_cows: Vec<TokenStream> = pk_columns
            .iter()
            .map(|col| quote! { ::std::borrow::Cow::Borrowed(#col) })
            .collect();
        quote! {
            /// Primary key definition
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> = {
                const PK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#pk_col_cows),*];
                ::std::option::Option::Some(#primary_key_def::new(#table_name, #pk_name).columns(PK_COLS))
            };
        }
    };

    // Build foreign key DDL definitions
    let mut fk_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            field.foreign_key.as_ref().map(|fk_ref| {
                let ref_table_ident = &fk_ref.table_ident;
                let ref_column = fk_ref.column_ident.to_string().to_snake_case();
                let fk_name = format!("{}_{}_fkey", table_name, field.column_name);
                let column_name = &field.column_name;

                let mut modifiers = Vec::new();
                if let Some(ref on_delete) = fk_ref.on_delete {
                    let action_token = referential_action_token(on_delete, &referential_action);
                    modifiers.push(
                        quote! { .on_delete(#action_token) },
                    );
                }
                if let Some(ref on_update) = fk_ref.on_update {
                    let action_token = referential_action_token(on_update, &referential_action);
                    modifiers.push(
                        quote! { .on_update(#action_token) },
                    );
                }

                quote! {
                    {
                        const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                        const FK_REFS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#ref_column)];
                        #foreign_key_def::new(#table_name, #fk_name)
                            .columns(FK_COLS)
                            .references(<#ref_table_ident>::TABLE_NAME, FK_REFS)
                            #(#modifiers)*
                    }
                }
            })
        })
        .collect();

    for fk in &ctx.attrs.composite_foreign_keys {
        let ref_table_ident = &fk.target_table;
        let source_columns: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let target_columns: Vec<String> = fk
            .target_columns
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let fk_name = format!("{}_{}_fkey", table_name, source_columns.join("_"));
        let fk_cols: Vec<TokenStream> = source_columns
            .iter()
            .map(|c| quote! { ::std::borrow::Cow::Borrowed(#c) })
            .collect();
        let fk_ref_cols: Vec<TokenStream> = target_columns
            .iter()
            .map(|c| quote! { ::std::borrow::Cow::Borrowed(#c) })
            .collect();

        let mut modifiers = Vec::new();
        if let Some(ref on_delete) = fk.on_delete {
            let action_token = referential_action_token(on_delete.as_str(), &referential_action);
            modifiers.push(quote! { .on_delete(#action_token) });
        }
        if let Some(ref on_update) = fk.on_update {
            let action_token = referential_action_token(on_update.as_str(), &referential_action);
            modifiers.push(quote! { .on_update(#action_token) });
        }

        fk_defs.push(quote! {
            {
                const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#fk_cols),*];
                const FK_REF_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#fk_ref_cols),*];
                #foreign_key_def::new(#table_name, #fk_name)
                    .columns(FK_COLS)
                    .references(<#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
                    #(#modifiers)*
            }
        });
    }

    // Build unique constraint DDL definitions (for non-primary unique columns)
    let unique_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_unique())
        .map(|field| {
            let unique_name = format!("{}_{}_unique", table_name, field.column_name);
            let column_name = &field.column_name;

            quote! {
                {
                    const UQ_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                    #unique_constraint_def::new(#table_name, #unique_name).columns(UQ_COLS)
                }
            }
        })
        .collect();

    quote! {
        /// Const DDL table definition for compile-time schema metadata.
        pub const DDL_TABLE: #table_def =
            #table_def::new(#table_name)
            #(#table_modifiers)*;

        /// Const DDL column definitions for compile-time schema metadata.
        pub const DDL_COLUMNS: &'static [#column_def] = &[
            #(#column_defs),*
        ];

        #pk_def

        /// Foreign key definitions
        pub const DDL_FOREIGN_KEYS: &'static [#foreign_key_def] = &[
            #(#fk_defs),*
        ];

        /// Unique constraint definitions
        pub const DDL_UNIQUE_CONSTRAINTS: &'static [#unique_constraint_def] = &[
            #(#unique_defs),*
        ];

        /// Index definitions (defined via separate #[SQLiteIndex] structs)
        pub const DDL_INDEXES: &'static [#index_def] = &[];

        /// Generate the CREATE TABLE SQL using the DDL definitions.
        ///
        /// This is the single source of truth for SQL generation, building
        /// the statement from the const DDL entities above.
        pub fn create_table_sql() -> ::std::string::String {
            let table = Self::DDL_TABLE.into_table();
            let columns: ::std::vec::Vec<_> = Self::DDL_COLUMNS.iter().map(|c| c.into_column()).collect();
            let pk = Self::DDL_PRIMARY_KEY.map(|p| p.into_primary_key());
            let fks: ::std::vec::Vec<_> = Self::DDL_FOREIGN_KEYS.iter().map(|f| f.into_foreign_key()).collect();
            let uniques: ::std::vec::Vec<_> = Self::DDL_UNIQUE_CONSTRAINTS.iter().map(|u| u.into_unique_constraint()).collect();

            #table_sql::new(&table)
                .columns(&columns)
                .primary_key(pk.as_ref())
                .foreign_keys(&fks)
                .unique_constraints(&uniques)
                .create_table_sql()
        }

        /// Returns the DDL SQL for creating this table.
        pub fn ddl_sql() -> &'static str {
            <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::SQL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::column_to_sql;
    use crate::common::Constraint;
    use crate::sqlite::field::{FieldInfo, SQLiteType};

    fn text_field<'a>(
        ident: &'a syn::Ident,
        ty: &'a syn::Type,
        name: &str,
        default: Option<syn::Expr>,
        nullable: bool,
    ) -> FieldInfo<'a> {
        FieldInfo {
            ident,
            field_type: ty,
            base_type: ty,
            column_name: name.to_string(),
            sql_definition: String::new(),
            is_nullable: nullable,
            has_default: default.is_some(),
            is_autoincrement: false,
            is_json: false,
            is_enum: false,
            is_uuid: false,
            is_custom_type: false,
            column_type: SQLiteType::Text,
            foreign_key: None,
            constraint: Constraint::None,
            collate: None,
            default_value: default,
            default_fn: None,
            marker_exprs: Vec::new(),
            select_type: None,
            update_type: None,
        }
    }

    #[test]
    fn column_default_string_literals_escape_single_quotes() {
        let ident: syn::Ident = syn::parse_str("display_name").expect("valid ident");
        let ty: syn::Type = syn::parse_str("String").expect("valid type");
        let default_expr: syn::Expr = syn::parse_str("\"O'Hara\"").expect("valid expr");

        let field = text_field(&ident, &ty, "display_name", Some(default_expr), false);
        let sql = column_to_sql(&field, false, false);
        assert!(
            sql.contains("DEFAULT 'O''Hara'"),
            "expected escaped default string, got: {sql}"
        );
    }

    #[test]
    fn column_uses_backticks_and_skips_not_null_for_integer_pk() {
        // Matches Column::to_column_sql behavior: an INTEGER PRIMARY KEY
        // column should NOT have a NOT NULL constraint, because such columns
        // alias rowid and accept NULL on insert.
        let ident: syn::Ident = syn::parse_str("id").expect("valid ident");
        let ty: syn::Type = syn::parse_str("i32").expect("valid type");
        let mut field = text_field(&ident, &ty, "id", None, false);
        field.column_type = SQLiteType::Integer;
        field.constraint = Constraint::StandalonePrimaryKey;
        let sql = column_to_sql(
            &field, /*inline_pk=*/ true, /*inline_unique=*/ false,
        );
        assert_eq!(sql, "`id` INTEGER PRIMARY KEY");
    }

    #[test]
    fn column_text_not_null_emits_backticks_and_constraint() {
        let ident: syn::Ident = syn::parse_str("name").expect("valid ident");
        let ty: syn::Type = syn::parse_str("String").expect("valid type");
        let field = text_field(&ident, &ty, "name", None, false);
        let sql = column_to_sql(&field, false, false);
        assert_eq!(sql, "`name` TEXT NOT NULL");
    }

    #[test]
    fn column_collate_clause_follows_constraints() {
        let ident: syn::Ident = syn::parse_str("name").expect("valid ident");
        let ty: syn::Type = syn::parse_str("String").expect("valid type");
        let mut field = text_field(&ident, &ty, "name", None, false);
        field.collate = Some("NOCASE".to_string());
        let sql = column_to_sql(&field, false, false);
        assert_eq!(sql, "`name` TEXT NOT NULL COLLATE NOCASE");
    }
}
