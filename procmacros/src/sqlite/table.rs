use super::field::{FieldAttributes, TableField};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident};

/// Implementation of the SQLiteTable attribute macro
pub(crate) fn table_attr_macro(
    input: syn::DeriveInput,
    attrs: TokenStream,
) -> syn::Result<TokenStream> {
    // Parse the attribute tokens to extract the parameters
    let mut name_attr: Option<String> = None;
    let mut strict_attr = false;
    let mut without_rowid_attr = false;

    // Parse attribute tokens manually
    if !attrs.is_empty() {
        // Convert to string and parse manually
        let attr_str = attrs.to_string();

        // Simple parsing of name="value"
        if attr_str.contains("name") {
            // Very naive parsing just for this example
            if let Some(start) = attr_str.find("name") {
                if let Some(eq_pos) = attr_str[start..].find('=') {
                    let start_pos = start + eq_pos + 1;
                    if let Some(quote_pos) = attr_str[start_pos..].find('"') {
                        let value_start = start_pos + quote_pos + 1;
                        if let Some(end_quote_pos) = attr_str[value_start..].find('"') {
                            let value = &attr_str[value_start..(value_start + end_quote_pos)];
                            name_attr = Some(value.to_string());
                        }
                    }
                }
            }
        }

        // Check for strict flag
        if attr_str.contains("strict") {
            strict_attr = true;
        }

        // Check for without_rowid flag
        if attr_str.contains("without_rowid") {
            without_rowid_attr = true;
        }
    }

    let struct_name = &input.ident;

    // Use our extracted attribute values instead of the TableAttributes
    let table_name = name_attr.unwrap_or_else(|| struct_name.to_string().to_lowercase());
    let strict = strict_attr;
    let without_rowid = without_rowid_attr;

    // Continue with the existing implementation
    let fields = if let syn::Data::Struct(ref data) = input.data {
        data.fields
            .iter()
            .map(|field| {
                let ident = field
                    .ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(&field, "No field name."))?;

                let field_attributes = FieldAttributes::try_from(&field.attrs)?;

                Ok(TableField {
                    ident,
                    attrs: field_attributes,
                    field,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?
    } else {
        return Err(syn::Error::new_spanned(
            &input,
            "SQLiteTable can only be applied to structs.",
        ));
    };

    // Collect primary key columns
    let primary_key_fields: Vec<_> = fields
        .iter()
        .filter(|field| field.attrs.primary_key.is_some())
        .collect();

    // Validate autoincrement is only used on primary key fields
    for field in &fields {
        if let Some(ref auto) = field.attrs.autoincrement {
            if field.attrs.primary_key.is_none() {
                return Err(syn::Error::new_spanned(
                    auto,
                    "drizzle: 'auto increment' can only be assigned to 'primary key'.",
                ));
            }
        }
    }

    // Generate the SQL column definitions and field types
    let column_defs = fields
        .iter()
        .map(|field| {
            let ident = field.ident;
            let field_name = format_ident!("{}", ident);

            // Get the original field type
            let original_field_type = &field.field.ty;

            let column_name = match &field.attrs.name {
                Some(name) => name.clone(),
                None => field_name.to_string(),
            };

            let is_primary = field.attrs.primary_key.is_some();
            let is_autoincrement = field.attrs.autoincrement.is_some();
            let is_unique = field.attrs.unique.is_some();
            let is_nullable = super::field::is_option_type(&field.field.ty);

            // Determine if the field type is an enum that implements SQLiteEnum
            // This requires analyzing the inner type (for Option<T> we need to check T)
            let inner_type = if is_nullable {
                super::field::get_option_inner_type(&field.field.ty).unwrap_or(&field.field.ty)
            } else {
                &field.field.ty
            };

            // Get column type, checking if it's an enum type
            let column_type = match field.attrs.column_type.as_deref() {
                Some("integer") => "INTEGER",
                Some("real") => "REAL",
                Some("text") => "TEXT",
                Some("blob") => "BLOB",
                Some("number") => "NUMERIC",
                _ => "TEXT",
            };

            // Create column definition
            let mut sql = format!("{} {}", column_name, column_type);

            // Add column constraints
            if is_primary && primary_key_fields.len() <= 1 {
                sql.push_str(" PRIMARY KEY");
                if is_autoincrement {
                    sql.push_str(" AUTOINCREMENT");
                }
            }

            if !is_nullable {
                sql.push_str(" NOT NULL");
            }

            if is_unique {
                sql.push_str(" UNIQUE");
            }

            // Add default value
            if let Some(default) = &field.attrs.default_value {
                // For simple cases, format the default value
                if let syn::Expr::Lit(expr_lit) = default {
                    match &expr_lit.lit {
                        syn::Lit::Int(i) => sql.push_str(&format!(" DEFAULT {}", i)),
                        syn::Lit::Float(f) => sql.push_str(&format!(" DEFAULT {}", f)),
                        syn::Lit::Bool(b) => {
                            sql.push_str(&format!(" DEFAULT {}", if b.value { 1 } else { 0 }))
                        }
                        syn::Lit::Str(s) => sql.push_str(&format!(" DEFAULT '{}'", s.value())),
                        _ => {}
                    }
                }
            }

            let default_fn = if let Some(default) = &field.attrs.default_fn {
                quote! {
                    Some(#default)
                }
            } else {
                quote! { None }
            };

            // Generate the column constant
            let col_const_name = field_name.clone();
            let column_const = quote! {
                #[allow(non_upper_case_globals, dead_code)]
                // Assuming SQLiteColumn is in querybuilder::sqlite::query_builder
                pub const #col_const_name: ::drizzle_rs::SQLiteColumn<'a, #original_field_type, Self> =
                    ::drizzle_rs::SQLiteColumn::new(
                        #column_name,
                        #sql,
                        #default_fn
                    );
            };

            // Return the column definition, field definition, and SQL
            (
                column_const,
                quote! {
                    // Assuming SQLiteColumn is in querybuilder::sqlite::query_builder
                    pub #field_name: ::drizzle_rs::SQLiteColumn<'a, #original_field_type, Self>
                },
                sql,                          // SQL for CREATE TABLE
                field_name,                   // Field name as an ident
                original_field_type.clone(),  // Original field type
                column_name,                  // Column name as string
            )
        })
        .collect::<Vec<_>>();

    // Extract components from column_defs
    let column_consts: Vec<_> = column_defs
        .iter()
        .map(|(const_def, _, _, _, _, _)| const_def.clone())
        .collect();
    let field_defs: Vec<_> = column_defs
        .iter()
        .map(|(_, field_def, _, _, _, _)| field_def.clone())
        .collect();
    let field_sqls: Vec<_> = column_defs.iter().map(|(_, _, sql, _, _, _)| sql).collect();
    let field_names: Vec<_> = column_defs
        .iter()
        .map(|(_, _, _, name, _, _)| name)
        .collect();
    let field_types: Vec<_> = column_defs.iter().map(|(_, _, _, _, ty, _)| ty).collect();
    let column_names: Vec<_> = column_defs
        .iter()
        .map(|(_, _, _, _, _, col_name)| col_name)
        .collect();

    // Pair column names and SQL for TableColumns
    let column_name_sql_pairs = column_names.iter().zip(field_sqls.iter()).map(|(cn, fs)| {
        quote! { (#cn, #fs) }
    });

    // Define from_row fields before using them
    let from_row_fields = fields.iter().map(|field| {
        let field_name = &field.ident;
        let column_name: String = field
            .attrs
            .name
            .clone()
            .unwrap_or_else(|| field.ident.to_string());
        quote! {
            #field_name: row.get(#column_name)?
        }
    });

    // Build the CREATE TABLE SQL string
    let mut create_table_sql = format!("CREATE TABLE {} (", table_name);

    // Add field definitions
    if !field_sqls.is_empty() {
        create_table_sql.push_str(&field_sqls[0]);
        for sql in &field_sqls[1..] {
            create_table_sql.push_str(", ");
            create_table_sql.push_str(sql);
        }
    }

    // Add composite primary key if needed
    if primary_key_fields.len() > 1 {
        let primary_key_cols: Vec<_> = primary_key_fields
            .iter()
            .map(|field| {
                field
                    .attrs
                    .name
                    .as_ref()
                    .unwrap_or(&field.ident.to_string())
                    .clone()
            })
            .collect();
        create_table_sql.push_str(&format!(", PRIMARY KEY ({})", primary_key_cols.join(", ")));
    }

    create_table_sql.push(')');

    // Add STRICT if specified
    if strict {
        create_table_sql.push_str(" STRICT");
    }

    // Add WITHOUT ROWID if specified
    if without_rowid {
        create_table_sql.push_str(" WITHOUT ROWID");
    }

    create_table_sql.push(';');

    // Generate model types for Select, Insert, and Update
    let select_model_name = format_ident!("Select{}", struct_name);
    let insert_model_name = format_ident!("Insert{}", struct_name);
    let update_model_name = format_ident!("Update{}", struct_name);

    // Generate initialization code for Default implementation
    let init_fields = field_names.iter().map(|field_name| {
        quote! {
            #field_name: Self::#field_name
        }
    });

    // === MOVE IMPL GENERATION HERE (BEFORE `expanded`) ===
    // Generate implementation for SQLSchema
    let sql_schema_impl = quote! {
        impl<'a> ::drizzle_rs::SQLSchema<'a, ::drizzle_rs::SQLiteTableType> for #struct_name<'a> {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle_rs::SQLiteTableType = ::drizzle_rs::SQLiteTableType::Table;
            const SQL: &'a str = #create_table_sql;
            const ALIAS: Option<&'a str> = None;
        }
    };

    // Generate the struct with transformed field types and implementations
    let expanded = quote! {
        #[derive(Clone, Debug)]
        pub struct #struct_name<'a> {
            #(#field_defs),*
        }

        impl<'a> #struct_name<'a> {
            // Column constants
            #(#column_consts)*

            // SQL constant for creating the table
            pub const SQL: &'static str = #create_table_sql;

            // Helper method to get a qualified column name for SELECT queries
            pub fn column(&self, name: &str) -> String {
                format!("{}:{}", stringify!(#struct_name), name)
            }

            // Create a new QueryBuilder for this table
            pub fn query() -> ::drizzle_rs::QueryBuilder<'a, #struct_name<'a>> {
                ::drizzle_rs::QueryBuilder::new()
            }
        }

        // Interpolate the generated impls
        #sql_schema_impl

        // Implement Default for initialization
        impl<'a> Default for #struct_name<'a> {
            fn default() -> Self {
                Self {
                    #(#init_fields),*
                }
            }
        }

        // Generate model for SELECT queries
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct #select_model_name {
            #(pub #field_names: #field_types),*
        }

        // Generate model for INSERT queries
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct #insert_model_name {
            #(pub #field_names: #field_types),*
        }

        // Generate model for UPDATE queries
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct #update_model_name {
            #(pub #field_names: #field_types),*
        }

    };

    Ok(expanded)
}

fn generate_relationship_methods(struct_name: &Ident, fields: &[TableField]) -> TokenStream {
    let methods = fields.iter().filter_map(|field| {
        let field_ident = field.ident;

        // Handle path-based references
        if let Some(references_path) = &field.attrs.references_path {
            let path_expr = references_path;

            Some(quote! {
                pub fn #field_ident(&self) -> impl crate::prelude::query_builder::ForeignKey {
                    crate::prelude::query_builder::SQLChunk::new()
                        .add(crate::prelude::query_builder::SQL::Raw(format!(
                            "{}.{} REFERENCES {}.{}",
                            self.name(),
                            stringify!(#field_ident),
                            <#path_expr as crate::sqlite::prelude::Column>::table().name(),
                            <#path_expr as crate::sqlite::prelude::Column>::name()
                        )))
                }
            })
        } else {
            None
        }
    });

    quote! {
        impl<'a> #struct_name<'a> {
            #(#methods)*
        }
    }
}
