use crate::postgres::table::context::MacroContext;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates an aliased version of a PostgreSQL table struct
///
/// For a table `Users` with fields `id` and `name`, this generates:
/// - `AliasedUsers` struct with `AliasedUsersId` and `AliasedUsersName` fields
/// - Each aliased field contains the table alias name
/// - `Users::alias(name: &'static str) -> AliasedUsers` method
pub fn generate_aliased_table(ctx: &MacroContext) -> syn::Result<TokenStream> {
    let table_name = &ctx.struct_ident;
    let struct_vis = &ctx.struct_vis;
    let aliased_table_name = format_ident!("Aliased{}", table_name);

    // Generate aliased field structs and their names
    let aliased_fields: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|field| {
            let field_name = &field.ident;
            // Use same casing as original column types to avoid conflicts
            let field_name_pascal = field_name.to_string().to_upper_camel_case();
            let aliased_field_type = format_ident!("Aliased{}{}", table_name, field_name_pascal);

            (field_name, aliased_field_type)
        })
        .collect();

    // Generate the aliased field type definitions
    let aliased_field_definitions: Vec<TokenStream> = ctx.field_infos.iter().zip(aliased_fields.iter()).map(|(field, (_, aliased_field_type))| {
        let field_name = &field.ident;
        // Use the same naming pattern as original column types
        let field_name_pascal = field_name.to_string().to_upper_camel_case();
        let original_field_type = format_ident!("{}{}", table_name, field_name_pascal);

        quote! {
            #[allow(non_upper_case_globals, dead_code)]
            #[derive(Debug, Clone, Default)]
            #struct_vis struct #aliased_field_type {
                alias: &'static str,
            }

            impl #aliased_field_type {
                pub const fn new(alias: &'static str) -> Self {
                    Self { alias }
                }
            }

            // Implement column info traits for the aliased field
            impl SQLColumnInfo for #aliased_field_type {
                fn is_not_null(&self) -> bool {
                    // Forward to the original field instance with explicit trait qualification
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::is_not_null(&ORIGINAL_FIELD)
                }

                fn is_primary_key(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::is_primary_key(&ORIGINAL_FIELD)
                }

                fn is_unique(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::is_unique(&ORIGINAL_FIELD)
                }

                fn name(&self) -> &str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::name(&ORIGINAL_FIELD)
                }

                fn r#type(&self) -> &str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::r#type(&ORIGINAL_FIELD)
                }

                fn has_default(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::has_default(&ORIGINAL_FIELD)
                }

                fn table(&self) -> &dyn SQLTableInfo {
                    // This is tricky - we need a static reference but each column has different alias
                    // For now, return the original table info
                    static ORIGINAL_TABLE: #table_name = #table_name::new();
                    &ORIGINAL_TABLE
                }
            }

            // Implement PostgreSQL-specific column info traits
            impl PostgresColumnInfo for #aliased_field_type {
                fn is_serial(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_serial(&ORIGINAL_FIELD)
                }

                fn is_bigserial(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_bigserial(&ORIGINAL_FIELD)
                }

                fn is_generated_identity(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_generated_identity(&ORIGINAL_FIELD)
                }

                fn postgres_type(&self) -> &'static str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::postgres_type(&ORIGINAL_FIELD)
                }

                fn table(&self) -> &dyn PostgresTableInfo {
                    // This is tricky - we need a static reference but each alias instance
                    // has a different alias string. For now, return the original table info.
                    static ORIGINAL_TABLE: #table_name = #table_name::new();
                    &ORIGINAL_TABLE
                }

                fn foreign_key(&self) -> Option<&'static dyn PostgresColumnInfo> {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::foreign_key(&ORIGINAL_FIELD)
                }
            }

            // Column info is provided directly via PostgresColumnInfo::as_postgres_column
            // Implement SQLColumn trait for aliased field
            impl<'a> SQLColumn<'a, PostgresValue<'a>> for #aliased_field_type {
                type Table = #aliased_table_name;
                type TableType = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::TableType;
                type Type = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::Type;

                const PRIMARY_KEY: bool = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::PRIMARY_KEY;
                const NOT_NULL: bool = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::NOT_NULL;
                const UNIQUE: bool = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::UNIQUE;
                const DEFAULT: Option<Self::Type> = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::DEFAULT;

                fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    ORIGINAL_FIELD.default_fn()
                }
            }

            // Implement PostgresColumn trait for aliased field
            impl<'a> PostgresColumn<'a> for #aliased_field_type {
                // PostgreSQL columns don't have autoincrement like SQLite, but may have other specific traits
            }

            // Implement SQLSchema trait for aliased field
            impl<'a> SQLSchema<'a, &'a str, PostgresValue<'a>> for #aliased_field_type {
                const NAME: &'a str = <#original_field_type as SQLSchema<'a, &'a str, PostgresValue<'a>>>::NAME;
                const TYPE: &'a str = <#original_field_type as SQLSchema<'a, &'a str, PostgresValue<'a>>>::TYPE;
                const SQL: &'static str = <#original_field_type as SQLSchema<'a, &'a str, PostgresValue<'a>>>::SQL;
            }
            
            // ToSQL implementation that uses the alias
            impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for #aliased_field_type {
                fn to_sql(&self) -> SQL<'a, V> {
                    use SQLColumnInfo;
                    SQL::raw(format!(r#""{}"."{}""#, self.alias, self.name()))
                }
            }
        }
    }).collect();

    // Generate the aliased table struct fields
    let aliased_struct_fields: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, aliased_type)| {
            quote! {
                #struct_vis #field_name: #aliased_type
            }
        })
        .collect();

    // Generate field initializers for the alias() method
    let field_initializers: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, aliased_type)| {
            quote! {
                #field_name: #aliased_type::new(alias)
            }
        })
        .collect();

    Ok(quote! {

        // Generate all aliased field type definitions
        #(#aliased_field_definitions)*

        // Generate the aliased table struct
        #[allow(non_upper_case_globals, dead_code)]
        #[derive(Debug, Clone, Default)]
        #struct_vis struct #aliased_table_name {
            alias: &'static str,
            #(#aliased_struct_fields),*
        }

        impl #aliased_table_name {
            pub const fn new(alias: &'static str) -> Self {
                Self {
                    alias,
                    #(#field_initializers),*
                }
            }

        }


        // Implement table traits for the aliased table
        impl SQLTableInfo for #aliased_table_name {
            fn name(&self) -> &str {
                self.alias
            }

            fn columns(&self) -> &'static [&'static dyn SQLColumnInfo] {
                // TODO: This is tricky because we need static references but each alias instance
                // has a different alias string. For now, return original columns.
                // The individual aliased fields can still be accessed directly via table.field
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::columns(&ORIGINAL_TABLE)
            }

            fn dependencies(&self) -> Box<[&'static dyn SQLTableInfo]> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::dependencies(&ORIGINAL_TABLE)
            }
        }

        // Implement PostgreSQL-specific table traits for aliased table
        impl PostgresTableInfo for #aliased_table_name {
            fn r#type(&self) -> &PostgresSchemaType {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.r#type()
            }

            fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo] {
                // TODO: This is tricky because we need static references but each alias instance
                // has a different alias string. For now, return original columns.
                // The individual aliased fields can still be accessed directly via table.field
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as PostgresTableInfo>::postgres_columns(&ORIGINAL_TABLE)
            }

            fn postgres_dependencies(&self) -> Box<[&'static dyn PostgresTableInfo]> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as PostgresTableInfo>::postgres_dependencies(&ORIGINAL_TABLE)
            }
        }

        impl<'a> PostgresTable<'a> for #aliased_table_name {
            // PostgreSQL tables don't have WITHOUT_ROWID or STRICT like SQLite
        }

        // Implement core SQLTable trait for aliased table
        impl<'a> SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> for #aliased_table_name {
            type Select = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Select;
            type Insert<T> = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Insert<T>;
            type Update = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Update;
            // Aliased tables alias to themselves (aliasing an already aliased table returns the same type)
            type Aliased = #aliased_table_name;

            fn alias(name: &'static str) -> Self::Aliased {
                #aliased_table_name::new(name)
            }
        }

        // Implement SQLSchema trait for aliased table
        impl<'a> SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>> for #aliased_table_name {
            const NAME: &'a str = <#table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::NAME;
            const TYPE: PostgresSchemaType = <#table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::TYPE;
            const SQL: &'static str = <#table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::SQL;
        }

        // ToSQL implementation for aliased table
        impl<'a> ToSQL<'a, PostgresValue<'a>> for #aliased_table_name {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.to_sql().alias(self.alias)
            }
        }

        // Add alias() method to the original table struct
        impl #table_name {
            pub const fn alias(alias: &'static str) -> #aliased_table_name {
                #aliased_table_name::new(alias)
            }
        }
    })
}
