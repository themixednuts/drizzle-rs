use crate::generators::{generate_impl, generate_sql_column_info, generate_sql_table_info};
use crate::sqlite::generators::*;
use crate::sqlite::table::context::MacroContext;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates an aliased version of a table struct
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
    let aliased_field_definitions: Vec<TokenStream> = ctx.field_infos.iter().zip(aliased_fields.iter()).map(|(field, (_, aliased_field_type))| -> syn::Result<TokenStream> {
        let field_name = &field.ident;
        // Use the same naming pattern as original column types
        let field_name_pascal = field_name.to_string().to_upper_camel_case();
        let original_field_type = format_ident!("{}{}", table_name, field_name_pascal);
        // Generate struct definition
        let struct_def = quote! {
            #[allow(non_upper_case_globals, dead_code)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #aliased_field_type {
                alias: &'static str,
            }
        };

        // Generate constructor impl
        let impl_new = generate_impl(&aliased_field_type, quote! {
            pub const fn new(alias: &'static str) -> Self {
                Self { alias }
            }
        });

        let sqlite_column_info_impl = generate_sqlite_column_info(aliased_field_type,
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as drizzle_sqlite::traits::SQLiteColumnInfo>::is_autoincrement(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as drizzle_sqlite::traits::SQLiteColumnInfo>::table(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as drizzle_sqlite::traits::SQLiteColumnInfo>::foreign_key(&ORIGINAL_FIELD)
            }
        );


        // Generate ToSQL implementation that uses the alias
        let to_sql_custom_impl = quote! {
            impl<'a, V: ::drizzle_core::SQLParam + 'a> ::drizzle_core::ToSQL<'a, V> for #aliased_field_type {
                fn to_sql(&self) -> ::drizzle_core::SQL<'a, V> {
                    ::drizzle_core::SQL::raw(format!(r#""{}"."{}""#, self.alias, ::drizzle_core::SQLColumnInfo::name(self)))
                }
            }
        };

        // Use generators for trait implementations
        let sql_column_info_impl = generate_sql_column_info(&aliased_field_type,
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::name(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::r#type(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::is_primary_key(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::is_not_null(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::is_unique(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::has_default(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as ::drizzle_core::SQLColumnInfo>::foreign_key(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                &ORIGINAL_TABLE
            },
        );
        let sql_column_impl = generate_sql_column(aliased_field_type,
            quote! {#aliased_table_name},
            quote! {<#original_field_type as ::drizzle_core::SQLColumn<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>>::TableType},
            quote! {<#original_field_type as ::drizzle_core::SQLColumn<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>>::Type},
            quote! {<#original_field_type as ::drizzle_core::SQLColumn<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>>::PRIMARY_KEY},
            quote! {<#original_field_type as ::drizzle_core::SQLColumn<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>>::NOT_NULL},
            quote! {<#original_field_type as ::drizzle_core::SQLColumn<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>>::UNIQUE},
            quote! {<#original_field_type as ::drizzle_core::SQLColumn<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>>::DEFAULT},
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                ORIGINAL_FIELD.default_fn()
            }
        );
        let sqlite_column_impl = generate_sqlite_column(&aliased_field_type, quote! {
            <#original_field_type as drizzle_sqlite::traits::SQLiteColumn<'a>>::AUTOINCREMENT
        });
        let sql_schema_field_impl = generate_sql_schema_field(aliased_field_type,
            quote! {<#original_field_type as ::drizzle_core::SQLSchema<'a, &'a str, ::drizzle_sqlite::values::SQLiteValue<'a>>>::NAME},
            quote! {<#original_field_type as ::drizzle_core::SQLSchema<'a, &'a str, ::drizzle_sqlite::values::SQLiteValue<'a>>>::TYPE},
            quote! {<#original_field_type as ::drizzle_core::SQLSchema<'a, &'a str, ::drizzle_sqlite::values::SQLiteValue<'a>>>::SQL}
        );

        let into_sqlite_value_impl = quote! {
            impl<'a> ::std::convert::Into<::drizzle_sqlite::values::SQLiteValue<'a>> for #aliased_field_type {
                fn into(self) -> ::drizzle_sqlite::values::SQLiteValue<'a> {
                    let column_ref = format!(r#""{}"."{}""#, self.alias, ::drizzle_core::SQLColumnInfo::name(&self));
                    ::drizzle_sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Owned(column_ref))
                }
            }
        };

        Ok(quote! {
            #struct_def
            #impl_new
            #sql_column_info_impl
            #sqlite_column_info_impl
            #sql_column_impl
            #sqlite_column_impl
            #sql_schema_field_impl
            #to_sql_custom_impl
            #into_sqlite_value_impl
        })
    }).collect::<syn::Result<_>>()?;

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

    let sql_table_info = generate_sql_table_info(
        &aliased_table_name,
        quote! {self.alias},
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as ::drizzle_core::SQLTableInfo>::columns(&ORIGINAL_TABLE)
        },
    );

    let sqlite_table_info = generate_sqlite_table_info(
        &aliased_table_name,
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            drizzle_sqlite::traits::SQLiteTableInfo::r#type(&ORIGINAL_TABLE)
        },
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            drizzle_sqlite::traits::SQLiteTableInfo::strict(&ORIGINAL_TABLE)
        },
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            drizzle_sqlite::traits::SQLiteTableInfo::without_rowid(&ORIGINAL_TABLE)
        },
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as drizzle_sqlite::traits::SQLiteTableInfo>::columns(&ORIGINAL_TABLE)
        },
    );

    let sql_table = generate_sql_table(
        &aliased_table_name,
        quote! {<#table_name as ::drizzle_core::SQLTable<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::Select},
        quote! {<#table_name as ::drizzle_core::SQLTable<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::Insert<T>},
        quote! {<#table_name as ::drizzle_core::SQLTable<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::Update},
        // Aliased tables alias to themselves (aliasing an already aliased table returns the same type)
        quote! {#aliased_table_name},
    );

    let sqlite_table = generate_sqlite_table(
        &aliased_table_name,
        quote! {<#table_name as drizzle_sqlite::traits::SQLiteTable<'a>>::WITHOUT_ROWID},
        quote! {<#table_name as drizzle_sqlite::traits::SQLiteTable<'a>>::STRICT},
    );

    let sql_schema = generate_sql_schema(
        &aliased_table_name,
        quote! {<#table_name as ::drizzle_core::SQLSchema<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::NAME},
        quote! {<#table_name as ::drizzle_core::SQLSchema<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::TYPE},
        quote! {<#table_name as ::drizzle_core::SQLSchema<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::SQL},
        Some(quote! {
            {
                static INSTANCE: #table_name = #table_name::new();
                <#table_name as ::drizzle_core::SQLSchema<'a, ::drizzle_sqlite::common::SQLiteSchemaType, ::drizzle_sqlite::values::SQLiteValue<'a>>>::sql(&INSTANCE)
            }
        }),
    );

    let to_sql_impl = generate_to_sql(
        &aliased_table_name,
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            ORIGINAL_TABLE.to_sql().alias(self.alias)
        },
    );

    Ok(quote! {

        // Generate all aliased field type definitions
        #(#aliased_field_definitions)*

        // Generate the aliased table struct
        #[allow(non_upper_case_globals, dead_code)]
        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
        #sql_table_info

        // Implement SQLite-specific table traits for aliased table
        #sqlite_table_info

        // Implement core SQLTable trait for aliased table
        #sql_table

        #sqlite_table

        // Implement SQLSchema trait for aliased table
        #sql_schema

        // ToSQL implementation for aliased table
        #to_sql_impl

        // Add alias() method to the original table struct
        impl #table_name {
            pub const fn alias(alias: &'static str) -> #aliased_table_name {
                #aliased_table_name::new(alias)
            }
        }
    })
}
