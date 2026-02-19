use crate::common::generate_expr_impl;
use crate::generators::{
    SQLTableInfoConfig, generate_impl, generate_sql_column_info, generate_sql_table_info,
};
use crate::paths::{core as core_paths, sqlite as sqlite_paths, std as std_paths};
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
/// - `Users::alias::<Tag>() -> UsersAlias<Tag>` method
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
    let sql_table_meta = core_paths::sql_table_meta();
    let alias_tag = core_paths::tag();
    let taggable_alias = core_paths::taggable_alias();
    let tagged = core_paths::tagged();
    let sql_table_info = core_paths::sql_table_info();
    let token = core_paths::token();
    let to_sql = core_paths::to_sql();
    let sql_param = core_paths::sql_param();
    let phantom_data = std_paths::phantom_data();
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
                    #sql::ident(self.alias)
                        .push(#token::DOT)
                        .append(#sql::ident({
                            static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                            #sql_column_info::name(&ORIGINAL_FIELD)
                        }))
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
            quote! {<#original_field_type as #sql_column<'a, #sqlite_value<'a>>>::ForeignKeys},
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

        let expr_value_type = core_paths::expr_value_type();
        let into_select_target = core_paths::into_select_target();
        let select_cols = core_paths::select_cols();

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
            impl #expr_value_type for #aliased_field_type {
                type ValueType = <#original_field_type as #expr_value_type>::ValueType;
            }
            impl #into_select_target for #aliased_field_type {
                type Marker = #select_cols<(#aliased_field_type,)>;
            }
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

    let sql_table_info_impl = generate_sql_table_info(SQLTableInfoConfig {
        struct_ident: &aliased_table_name,
        name: quote! {self.alias},
        schema: quote! { ::std::option::Option::None },
        columns: quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sql_table_info>::columns(&ORIGINAL_TABLE)
        },
        primary_key: quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sql_table_info>::primary_key(&ORIGINAL_TABLE)
        },
        foreign_keys: quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sql_table_info>::foreign_keys(&ORIGINAL_TABLE)
        },
        constraints: quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sql_table_info>::constraints(&ORIGINAL_TABLE)
        },
        dependencies: quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sql_table_info>::dependencies(&ORIGINAL_TABLE)
        },
    });

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
        quote! {
            static ORIGINAL_TABLE: #table_name = #table_name::new();
            <#table_name as #sqlite_table_info>::sqlite_dependencies(&ORIGINAL_TABLE)
        },
    );

    let sql_table_impl = generate_sql_table(SQLTableConfig {
        struct_ident: &aliased_table_name,
        select: quote! {<#table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Select},
        insert: quote! {<#table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Insert<T>},
        update: quote! {<#table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Update},
        // Aliased tables alias to themselves (aliasing an already aliased table returns the same type)
        aliased: quote! {#aliased_table_name},
        foreign_keys: quote! {<#table_name as #sql_table_meta>::ForeignKeys},
        primary_key: quote! {<#table_name as #sql_table_meta>::PrimaryKey},
        constraints: quote! {<#table_name as #sql_table_meta>::Constraints},
    });

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
                <#table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::ddl(&INSTANCE)
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

    let tagged_alias_meta_name = format_ident!("AliasTagMeta{}", table_name);
    let alias_type_name = format_ident!("{}Alias", table_name);
    let tagged_const_defs: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, aliased_type)| {
            quote! {
                const #field_name: #aliased_type = #aliased_type::new(Tag::NAME);
            }
        })
        .collect();
    let tagged_sql_column_refs: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, _)| {
            quote! {
                &(#tagged_alias_meta_name::<Tag>::#field_name) as &'static dyn #sql_column_info
            }
        })
        .collect();
    let tagged_sqlite_column_refs: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, _)| {
            quote! {
                &(#tagged_alias_meta_name::<Tag>::#field_name) as &'static dyn #sqlite_column_info
            }
        })
        .collect();

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

        #struct_vis struct #alias_type_name<Tag: #alias_tag>(#tagged<#aliased_table_name, Tag>);

        impl<Tag: #alias_tag> #alias_type_name<Tag> {
            pub const fn new() -> Self {
                Self(#tagged::new(#aliased_table_name::new(Tag::NAME)))
            }

            pub const fn from_inner(inner: #aliased_table_name) -> Self {
                Self(#tagged::new(inner))
            }
        }

        impl<Tag: #alias_tag> ::core::marker::Copy for #alias_type_name<Tag> {}

        impl<Tag: #alias_tag> ::core::clone::Clone for #alias_type_name<Tag> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<Tag: #alias_tag> ::core::default::Default for #alias_type_name<Tag> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<Tag: #alias_tag> ::core::ops::Deref for #alias_type_name<Tag> {
            type Target = #aliased_table_name;

            fn deref(&self) -> &Self::Target {
                ::core::ops::Deref::deref(&self.0)
            }
        }

        struct #tagged_alias_meta_name<Tag: #alias_tag>(#phantom_data<fn() -> Tag>);

        #[allow(non_upper_case_globals)]
        impl<Tag: #alias_tag> #tagged_alias_meta_name<Tag> {
            #(#tagged_const_defs)*

            const SQL_COLUMNS: &'static [&'static dyn #sql_column_info] = &[
                #(#tagged_sql_column_refs,)*
            ];

            const SQLITE_COLUMNS: &'static [&'static dyn #sqlite_column_info] = &[
                #(#tagged_sqlite_column_refs,)*
            ];
        }

        impl #taggable_alias for #aliased_table_name {
            type Tagged<Tag: #alias_tag> = #alias_type_name<Tag>;

            fn tag<Tag: #alias_tag>(self) -> Self::Tagged<Tag> {
                #alias_type_name::<Tag>::from_inner(self)
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

        impl<'a, Tag: #alias_tag> #to_sql<'a, #sqlite_value<'a>> for #alias_type_name<Tag> {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #to_sql::to_sql(::core::ops::Deref::deref(self))
            }
        }

        impl<Tag: #alias_tag + 'static> #sql_table_info for #alias_type_name<Tag> {
            fn name(&self) -> &str {
                Tag::NAME
            }

            fn schema(&self) -> ::std::option::Option<&str> {
                #sql_table_info::schema(::core::ops::Deref::deref(self))
            }

            fn columns(&self) -> &'static [&'static dyn #sql_column_info] {
                #tagged_alias_meta_name::<Tag>::SQL_COLUMNS
            }

            fn primary_key(&self) -> ::std::option::Option<&'static dyn drizzle::core::SQLPrimaryKeyInfo> {
                #sql_table_info::primary_key(::core::ops::Deref::deref(self))
            }

            fn foreign_keys(&self) -> &'static [&'static dyn drizzle::core::SQLForeignKeyInfo] {
                #sql_table_info::foreign_keys(::core::ops::Deref::deref(self))
            }

            fn constraints(&self) -> &'static [&'static dyn drizzle::core::SQLConstraintInfo] {
                #sql_table_info::constraints(::core::ops::Deref::deref(self))
            }

            fn dependencies(&self) -> &'static [&'static dyn #sql_table_info] {
                #sql_table_info::dependencies(::core::ops::Deref::deref(self))
            }
        }

        impl<Tag: #alias_tag + 'static> #sqlite_table_info for #alias_type_name<Tag> {
            fn r#type(&self) -> &#sqlite_schema_type {
                #sqlite_table_info::r#type(::core::ops::Deref::deref(self))
            }

            fn strict(&self) -> bool {
                #sqlite_table_info::strict(::core::ops::Deref::deref(self))
            }

            fn without_rowid(&self) -> bool {
                #sqlite_table_info::without_rowid(::core::ops::Deref::deref(self))
            }

            fn sqlite_columns(&self) -> &'static [&'static dyn #sqlite_column_info] {
                #tagged_alias_meta_name::<Tag>::SQLITE_COLUMNS
            }

            fn sqlite_dependencies(&self) -> &'static [&'static dyn #sqlite_table_info] {
                #sqlite_table_info::sqlite_dependencies(::core::ops::Deref::deref(self))
            }
        }

        impl<'a, Tag: #alias_tag + 'static> #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>> for #alias_type_name<Tag> {
            type Select = <#aliased_table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Select;
            type Insert<T> = <#aliased_table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Insert<T>;
            type Update = <#aliased_table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Update;
            type Aliased = <#aliased_table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::Aliased;
            type ForeignKeys = <#aliased_table_name as #sql_table_meta>::ForeignKeys;
            type PrimaryKey = <#aliased_table_name as #sql_table_meta>::PrimaryKey;
            type Constraints = <#aliased_table_name as #sql_table_meta>::Constraints;

            fn alias_named(name: &'static str) -> Self::Aliased {
                <#aliased_table_name as #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>>>::alias_named(name)
            }
        }

        impl<'a, Tag: #alias_tag + 'static> #sqlite_table<'a> for #alias_type_name<Tag> {
            const WITHOUT_ROWID: bool = <#aliased_table_name as #sqlite_table<'a>>::WITHOUT_ROWID;
            const STRICT: bool = <#aliased_table_name as #sqlite_table<'a>>::STRICT;
        }

        impl<'a, Tag: #alias_tag + 'static> #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>> for #alias_type_name<Tag> {
            const NAME: &'static str = <#aliased_table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::NAME;
            const TYPE: #sqlite_schema_type = <#aliased_table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::TYPE;
            const SQL: &'static str = <#aliased_table_name as #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>>>::SQL;

            fn ddl(&self) -> #sql<'a, #sqlite_value<'a>> {
                #sql_schema::ddl(::core::ops::Deref::deref(self))
            }
        }

        impl<Tag: #alias_tag + 'static> drizzle::core::HasSelectModel for #alias_type_name<Tag> {
            type SelectModel = <#aliased_table_name as drizzle::core::HasSelectModel>::SelectModel;
            const COLUMN_COUNT: usize = <#aliased_table_name as drizzle::core::HasSelectModel>::COLUMN_COUNT;
        }

        impl<Tag: #alias_tag + 'static> drizzle::core::IntoSelectTarget for #alias_type_name<Tag> {
            type Marker = drizzle::core::SelectStar;
        }

        // HasSelectModel for aliased table (delegates to original)
        impl drizzle::core::HasSelectModel for #aliased_table_name {
            type SelectModel = <#table_name as drizzle::core::HasSelectModel>::SelectModel;
            const COLUMN_COUNT: usize = <#table_name as drizzle::core::HasSelectModel>::COLUMN_COUNT;
        }
        impl drizzle::core::IntoSelectTarget for #aliased_table_name {
            type Marker = drizzle::core::SelectStar;
        }

        // Add alias() method to the original table struct
        impl #table_name {
            pub const fn alias<Tag: #alias_tag + 'static>() -> #alias_type_name<Tag> {
                #alias_type_name::<Tag>::new()
            }
        }
    })
}
