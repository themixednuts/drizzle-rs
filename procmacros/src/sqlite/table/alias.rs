use crate::common::generate_expr_impl;
use crate::generators::{generate_impl, generate_sql_column_info, generate_sql_table_info};
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
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

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_column = core_paths::sql_column();
    let sql_column_info = core_paths::sql_column_info();
    let sql_schema = core_paths::sql_schema();
    let sql_table = core_paths::sql_table();
    let sql_table_info = core_paths::sql_table_info();
    let to_sql = core_paths::to_sql();
    let sql_param = core_paths::sql_param();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_table = sqlite_paths::sqlite_table();
    let sqlite_table_info = sqlite_paths::sqlite_table_info();
    let sqlite_column = sqlite_paths::sqlite_column();
    let sqlite_column_info = sqlite_paths::sqlite_column_info();

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
        let impl_new = generate_impl(aliased_field_type, quote! {
            pub const fn new(alias: &'static str) -> Self {
                Self { alias }
            }
        });

        let sqlite_column_info_impl = generate_sqlite_column_info(aliased_field_type,
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sqlite_column_info>::is_autoincrement(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sqlite_column_info>::table(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sqlite_column_info>::foreign_key(&ORIGINAL_FIELD)
            }
        );


        // Generate ToSQL implementation that uses the alias
        let to_sql_custom_impl = quote! {
            impl<'a, V: #sql_param + 'a> #to_sql<'a, V> for #aliased_field_type {
                fn to_sql(&self) -> #sql<'a, V> {
                    #sql::raw(::std::format!(r#""{}"."{}""#, self.alias, #sql_column_info::name(self)))
                }
            }
        };

        // Use generators for trait implementations
        let sql_column_info_impl = generate_sql_column_info(aliased_field_type,
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::name(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::r#type(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::is_primary_key(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::is_not_null(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::is_unique(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::has_default(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                <#original_field_type as #sql_column_info>::foreign_key(&ORIGINAL_FIELD)
            },
            quote! {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                &ORIGINAL_TABLE
            },
        );
        let sql_column_impl = generate_sql_column(aliased_field_type,
            quote! {#aliased_table_name},
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::TableType},
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::Type},
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::PRIMARY_KEY},
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::NOT_NULL},
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::UNIQUE},
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::DEFAULT},
            quote! {
                static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                ORIGINAL_FIELD.default_fn()
            }
        );
        let sqlite_column_impl = generate_sqlite_column(aliased_field_type, quote! {
            <#original_field_type as #sqlite_column<'a>>::AUTOINCREMENT
        });
        let sql_schema_field_impl = generate_sql_schema_field(aliased_field_type,
            quote! {<#original_field_type as #sql_schema<'a, &'a str, #sqlite_value<'a>>>::NAME},
            quote! {<#original_field_type as #sql_schema<'a, &'a str, #sqlite_value<'a>>>::TYPE},
            quote! {<#original_field_type as #sql_schema<'a, &'a str, #sqlite_value<'a>>>::SQL}
        );

        let into_sqlite_value_impl = quote! {
            impl<'a> ::std::convert::Into<#sqlite_value<'a>> for #aliased_field_type {
                fn into(self) -> #sqlite_value<'a> {
                    let column_ref = ::std::format!(r#""{}"."{}""#, self.alias, #sql_column_info::name(&self));
                    #sqlite_value::Text(::std::borrow::Cow::Owned(column_ref))
                }
            }
        };

        // Generate Expr impl inheriting types from original column
        let expr = crate::paths::core::expr();
        let expr_impl = generate_expr_impl(
            aliased_field_type,
            sqlite_value.clone(),
            quote! {<#original_field_type as #expr::Expr<'a, #sqlite_value<'a>>>::SQLType},
            quote! {<#original_field_type as #expr::Expr<'a, #sqlite_value<'a>>>::Nullable},
        );

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
            #expr_impl
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

    let sql_table_info_impl = generate_sql_table_info(
        &aliased_table_name,
        quote! {self.alias},
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sql_table_info>::columns(&ORIGINAL_TABLE)
        },
    );

    let sqlite_table_info_impl = generate_sqlite_table_info(
        &aliased_table_name,
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            #sqlite_table_info::r#type(&ORIGINAL_TABLE)
        },
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            #sqlite_table_info::strict(&ORIGINAL_TABLE)
        },
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            #sqlite_table_info::without_rowid(&ORIGINAL_TABLE)
        },
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sqlite_table_info>::sqlite_columns(&ORIGINAL_TABLE)
        },
    );

    let sql_table_impl = generate_sql_table(
        &aliased_table_name,
        quote! {<#table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Select},
        quote! {<#table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Insert<T>},
        quote! {<#table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Update},
        // Aliased tables alias to themselves (aliasing an already aliased table returns the same type)
        quote! {#aliased_table_name},
    );

    let sqlite_table_impl = generate_sqlite_table(
        &aliased_table_name,
        quote! {<#table_name as #sqlite_table<'a>>::WITHOUT_ROWID},
        quote! {<#table_name as #sqlite_table<'a>>::STRICT},
    );

    let sql_schema_impl = generate_sql_schema(
        &aliased_table_name,
        quote! {<#table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::NAME},
        quote! {<#table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::TYPE},
        quote! {<#table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::SQL},
        Some(quote! {
            {
                static INSTANCE: #table_name = #table_name::new();
                <#table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::sql(&INSTANCE)
            }
        }),
    );

    let to_sql_impl = generate_to_sql(
        &aliased_table_name,
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            #to_sql::to_sql(&ORIGINAL_TABLE).alias(self.alias)
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
        #sql_table_info_impl

        // Implement SQLite-specific table traits for aliased table
        #sqlite_table_info_impl

        // Implement core SQLTable trait for aliased table
        #sql_table_impl

        #sqlite_table_impl

        // Implement SQLSchema trait for aliased table
        #sql_schema_impl

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
