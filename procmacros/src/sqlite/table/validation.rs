use crate::sqlite::field::{FieldInfo, SQLiteType};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Expr;

pub fn validate_strict_affinity(field_infos: &[FieldInfo], strict: bool) -> syn::Result<()> {
    let mut errors: Vec<syn::Error> = Vec::new();

    for info in field_infos {
        if info.uses_sqlite_column_codec() {
            continue;
        }

        if strict && !info.column_type.is_strict_allowed() {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses `{}` affinity, which is not allowed in STRICT tables",
                    info.column_name, info.column_type
                ),
            ));
        }

        if !strict && matches!(info.column_type, SQLiteType::Any) {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses `ANY`, which is only allowed on STRICT tables; add `#[SQLiteTable(strict)]`",
                    info.column_name
                ),
            ));
        }
    }

    let mut iter = errors.into_iter();
    if let Some(mut first) = iter.next() {
        for err in iter {
            first.combine(err);
        }
        return Err(first);
    }

    Ok(())
}

/// Validates AUTOINCREMENT usage at macro-expansion time.
///
/// `SQLite` only allows `AUTOINCREMENT` on the single `INTEGER PRIMARY KEY`
/// column of a rowid table:
/// - non-INTEGER columns (or non-PK columns) cannot be AUTOINCREMENT
/// - `WITHOUT ROWID` tables cannot use AUTOINCREMENT at all
/// - composite primary keys cannot carry AUTOINCREMENT
pub fn validate_autoincrement(field_infos: &[FieldInfo], without_rowid: bool) -> syn::Result<()> {
    let mut errors: Vec<syn::Error> = Vec::new();

    for info in field_infos {
        if !info.is_autoincrement {
            continue;
        }

        if without_rowid {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses AUTOINCREMENT, which is not allowed on WITHOUT ROWID tables",
                    info.column_name
                ),
            ));
        }

        if !info.is_custom_type && !matches!(info.column_type, SQLiteType::Integer) {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses AUTOINCREMENT but has `{}` affinity; AUTOINCREMENT requires an INTEGER PRIMARY KEY column",
                    info.column_name, info.column_type
                ),
            ));
        }

        if !info.is_primary() {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses AUTOINCREMENT but is not the primary key; add `primary` to the column attributes",
                    info.column_name
                ),
            ));
        } else if !info.constraint.is_inline_primary() {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses AUTOINCREMENT inside a composite primary key; AUTOINCREMENT requires a single-column INTEGER PRIMARY KEY",
                    info.column_name
                ),
            ));
        }
    }

    let mut iter = errors.into_iter();
    if let Some(mut first) = iter.next() {
        for err in iter {
            first.combine(err);
        }
        return Err(first);
    }

    Ok(())
}

/// Generates compile-time validation blocks for default literals
pub fn generate_default_validations(field_infos: &[FieldInfo]) -> TokenStream {
    let validations: Vec<TokenStream> = field_infos
        .iter()
        .filter_map(|info| {
            if let Some(Expr::Lit(expr_lit)) = &info.default_value {
                let base_type_tokens = &info.base_type; // already a syn::Type
                let base_type: proc_macro2::TokenStream =
                    if base_type_tokens.to_token_stream().to_string() == "String" {
                        quote! { &str }
                    } else {
                        quote! { #base_type_tokens }
                    };
                Some(quote! {
                    // Compile-time validation: ensure default literal is compatible with field type
                    const _: () = {
                        // This will cause a compile error if the literal type doesn't match the field type
                        // For example: `let _: i32 = "string";` will fail at compile time
                        //              `let _: String = 42;` will fail at compile time
                        let _: #base_type = #expr_lit;
                    };
                })
            } else {
                None
            }
        })
        .collect();

    if validations.is_empty() {
        quote!() // No validations needed
    } else {
        quote! {
            // Default literal validations - these blocks ensure type compatibility at compile time
            #(#validations)*
        }
    }
}

/// Generates associated-type equality checks for explicit storage markers on
/// codec-owned columns.
///
/// `DrizzleSQLiteColumn` remains the storage authority. An explicit `TEXT` or
/// other affinity marker is accepted only when it agrees with the codec's
/// associated SQLite type.
pub fn generate_codec_storage_validations(field_infos: &[FieldInfo]) -> TokenStream {
    let validations = field_infos.iter().filter_map(|info| {
        if !info.uses_sqlite_column_codec() || !info.has_explicit_type {
            return None;
        }

        let sqlite_types = crate::paths::sqlite::types();
        let expected = match info.column_type {
            SQLiteType::Integer => quote!(#sqlite_types::Integer),
            SQLiteType::Text => quote!(#sqlite_types::Text),
            SQLiteType::Blob => quote!(#sqlite_types::Blob),
            SQLiteType::Real => quote!(#sqlite_types::Real),
            SQLiteType::Numeric => quote!(#sqlite_types::Numeric),
            SQLiteType::Any => quote!(#sqlite_types::Any),
        };
        let base_type = info.base_type;
        let drizzle_sqlite_column = crate::paths::sqlite::drizzle_sqlite_column();

        Some(quote! {
            const _: fn() = || {
                fn assert_column_storage<__T: #drizzle_sqlite_column<SQLType = #expected>>() {}
                assert_column_storage::<#base_type>();
            };
        })
    });

    quote! { #(#validations)* }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Constraint;

    fn autoincrement_field<'a>(
        ident: &'a syn::Ident,
        ty: &'a syn::Type,
        column_type: SQLiteType,
        constraint: Constraint,
    ) -> FieldInfo<'a> {
        FieldInfo {
            ident,
            field_type: ty,
            base_type: ty,
            column_name: ident.to_string(),
            sql_definition: String::new(),
            is_nullable: false,
            has_default: false,
            is_autoincrement: true,
            is_json: false,
            is_enum: false,
            is_uuid: false,
            has_explicit_type: false,
            is_custom_type: false,
            column_type,
            foreign_key: None,
            relation_name: None,
            constraint,
            collate: None,
            default_value: None,
            default_sql: None,
            default_fn: None,
            generated_column: None,
            check_constraint: None,
            marker_exprs: Vec::new(),
            select_type: None,
            update_type: None,
        }
    }

    #[test]
    fn autoincrement_on_integer_pk_is_valid() {
        let ident: syn::Ident = syn::parse_str("id").expect("ident");
        let ty: syn::Type = syn::parse_str("i64").expect("type");
        let field = autoincrement_field(
            &ident,
            &ty,
            SQLiteType::Integer,
            Constraint::StandalonePrimaryKey,
        );
        assert!(validate_autoincrement(&[field], false).is_ok());
    }

    #[test]
    fn autoincrement_on_text_pk_is_rejected() {
        let ident: syn::Ident = syn::parse_str("id").expect("ident");
        let ty: syn::Type = syn::parse_str("String").expect("type");
        let field = autoincrement_field(
            &ident,
            &ty,
            SQLiteType::Text,
            Constraint::StandalonePrimaryKey,
        );
        let err = validate_autoincrement(&[field], false).expect_err("must reject");
        assert!(err.to_string().contains("AUTOINCREMENT"), "got: {err}");
    }

    #[test]
    fn autoincrement_on_without_rowid_table_is_rejected() {
        let ident: syn::Ident = syn::parse_str("id").expect("ident");
        let ty: syn::Type = syn::parse_str("i64").expect("type");
        let field = autoincrement_field(
            &ident,
            &ty,
            SQLiteType::Integer,
            Constraint::StandalonePrimaryKey,
        );
        let err = validate_autoincrement(&[field], true).expect_err("must reject");
        assert!(err.to_string().contains("WITHOUT ROWID"), "got: {err}");
    }

    #[test]
    fn autoincrement_in_composite_pk_is_rejected() {
        let ident: syn::Ident = syn::parse_str("id").expect("ident");
        let ty: syn::Type = syn::parse_str("i64").expect("type");
        let field = autoincrement_field(
            &ident,
            &ty,
            SQLiteType::Integer,
            Constraint::CompositePrimaryKey,
        );
        let err = validate_autoincrement(&[field], false).expect_err("must reject");
        assert!(err.to_string().contains("composite"), "got: {err}");
    }
}
