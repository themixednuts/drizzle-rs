use crate::sqlite::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;

/// Generates runtime code to build CREATE TABLE SQL with foreign key support.
pub(crate) fn generate_create_table_sql_runtime(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> TokenStream {
    let column_defs: Vec<TokenStream> = field_infos
        .iter()
        .map(|info| {
            let base_def = &info.sql_definition;

            if let Some(ref fk) = info.foreign_key {
                // Generate runtime code to build foreign key constraint
                let table_ident = &fk.table_ident;
                let column_ident = &fk.column_ident;
                
                // Generate ON DELETE/ON UPDATE clauses if specified
                let on_delete_clause = fk.on_delete.as_ref().map(|action| format!(" ON DELETE {}", action)).unwrap_or_default();
                let on_update_clause = fk.on_update.as_ref().map(|action| format!(" ON UPDATE {}", action)).unwrap_or_default();

                quote! {
                    {
                        let base_def = #base_def;
                        let table_name = #table_ident::NAME.to_string();
                        let column_name = <_ as SQLColumnInfo>::name(&#table_ident::#column_ident).to_string();
                        format!("{} REFERENCES {}({}){}{}", base_def, table_name, column_name, #on_delete_clause, #on_update_clause)
                    }
                }
            } else {
                quote! { #base_def.to_string() }
            }
        })
        .collect();

    let table_name_str = table_name;
    let composite_pk_code = if is_composite_pk {
        let pk_columns: Vec<&String> = field_infos
            .iter()
            .filter(|info| info.is_primary)
            .map(|info| &info.column_name)
            .collect();

        quote! {
            column_defs_str.push_str(", PRIMARY KEY (");
            column_defs_str.push_str(&[#(#pk_columns),*].join(", "));
            column_defs_str.push_str(")");
        }
    } else {
        quote! {}
    };

    let without_rowid_code = if without_rowid {
        quote! { sql.push_str(" WITHOUT ROWID"); }
    } else {
        quote! {}
    };

    let strict_code = if strict {
        quote! { sql.push_str(" STRICT"); }
    } else {
        quote! {}
    };

    quote! {
        {
            let column_defs = vec![#(#column_defs),*];
            let mut column_defs_str = column_defs.join(", ");
            #composite_pk_code
            let mut sql = format!("CREATE TABLE \"{}\" ({})", #table_name_str, column_defs_str);
            #without_rowid_code
            #strict_code
            sql.push(';');
            sql
        }
    }
}

/// Generates the static `CREATE TABLE` SQL string (for tables without foreign keys).
pub(crate) fn generate_create_table_sql(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> String {
    let column_defs: Vec<_> = field_infos
        .iter()
        .map(|info| info.sql_definition.clone())
        .collect();

    let mut create_sql = format!(
        "CREATE TABLE \"{}\" ({})",
        table_name,
        column_defs.join(", ")
    );

    if is_composite_pk {
        let pk_cols = field_infos
            .iter()
            .filter(|info| info.is_primary)
            .map(|info| format!("\"{}\"", info.column_name))
            .collect::<Vec<_>>()
            .join(", ");
        create_sql.push_str(&format!(", PRIMARY KEY ({})", pk_cols));
    }

    // Don't add extra closing paren since it's already in the format string
    if without_rowid {
        create_sql.push_str(" WITHOUT ROWID");
    }
    if strict {
        create_sql.push_str(" STRICT");
    }
    create_sql.push(';');
    create_sql
}
