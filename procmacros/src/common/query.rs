//! Shared query API code generation for both SQLite and PostgreSQL.
//!
//! Generates relation ZSTs, `RelationDef` impls, accessor traits/impls,
//! type aliases, `FromJsonValue` impls, and column selectors from FK declarations.

use heck::ToSnakeCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Result, Visibility};

/// FK info extracted from field declarations.
pub(crate) struct FkInfo {
    /// Source column name (e.g., "author_id").
    pub source_column: String,
    /// Target table ident (e.g., `User`).
    pub target_table_ident: Ident,
    /// Target column ident (e.g., `id`).
    pub target_column_ident: Ident,
    /// Whether the source field is nullable (Option<T>).
    pub is_nullable: bool,
}

/// How an enum field is stored in the database.
#[derive(Clone, Copy)]
pub(crate) enum EnumStorage {
    /// Stored as INTEGER — deserialize via `TryFrom<i64>`.
    Integer,
    /// Stored as TEXT — deserialize via `FromStr`.
    Text,
}

/// Info about a field for generating FromJsonValue.
pub(crate) struct FieldJsonInfo {
    /// The field ident (e.g., `id`).
    pub ident: Ident,
    /// The column name in SQL (e.g., "id").
    pub column_name: String,
    /// Whether the field is nullable.
    pub is_nullable: bool,
    /// Whether the field is a UUID type.
    pub is_uuid: bool,
    /// If the field is an enum, how it is stored in the database.
    pub enum_storage: Option<EnumStorage>,
    /// The unwrapped base type (e.g., `i32` even if the field is `Option<i32>`).
    pub base_type: syn::Type,
}

/// Generates all query API code for a table.
///
/// Returns a `TokenStream` containing:
/// - `QueryTable` impl for the table ZST
/// - Forward relation items (ZST, RelationDef impl, accessor method, result accessor trait, type alias)
/// - Reverse relation items (ZST, RelationDef impl, accessor method, result accessor trait, type alias)
/// - `FromJsonValue` impl for the select model
/// - `FromJsonValue` impl for the partial select model
/// - Column selector struct and `.columns()` method
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_query_api(
    struct_ident: &Ident,
    struct_vis: &Visibility,
    table_name: &str,
    select_model_ident: &Ident,
    partial_select_model_ident: &Ident,
    fk_infos: &[FkInfo],
    field_json_infos: &[FieldJsonInfo],
    column_names: &[String],
) -> Result<TokenStream> {
    let mut tokens = TokenStream::new();

    // 1. Generate QueryTable impl (table name, column names, select model, partial select model)
    tokens.extend(generate_query_table(
        struct_ident,
        select_model_ident,
        partial_select_model_ident,
        table_name,
        column_names,
    ));

    // 2. Generate forward relations (from this table to target)
    tokens.extend(generate_forward_relations(
        struct_ident,
        struct_vis,
        fk_infos,
    ));

    // 3. Generate reverse relations (from target tables back to this table)
    tokens.extend(generate_reverse_relations(
        struct_ident,
        struct_vis,
        fk_infos,
    ));

    // 4. Generate FromJsonValue for the select model
    tokens.extend(generate_from_json_value(
        select_model_ident,
        field_json_infos,
    ));

    // 5. Generate FromJsonValue for the partial select model (all fields optional)
    tokens.extend(generate_partial_from_json_value(
        partial_select_model_ident,
        field_json_infos,
    ));

    // 6. Generate column selector struct and `.columns()` method
    tokens.extend(generate_column_selector(
        struct_ident,
        struct_vis,
        field_json_infos,
    ));

    Ok(tokens)
}

/// Generates `QueryTable` impl for the table ZST.
fn generate_query_table(
    struct_ident: &Ident,
    select_model_ident: &Ident,
    partial_select_model_ident: &Ident,
    table_name: &str,
    column_names: &[String],
) -> TokenStream {
    let column_name_literals: Vec<&str> = column_names.iter().map(|s| s.as_str()).collect();

    quote! {
        impl drizzle::core::query::QueryTable for #struct_ident {
            type Select = #select_model_ident;
            type PartialSelect = #partial_select_model_ident;
            const TABLE_NAME: &'static str = #table_name;
            const COLUMN_NAMES: &'static [&'static str] = &[#(#column_name_literals),*];
        }
    }
}

/// Derive the forward method name from a column name.
/// Strips `_id` suffix: `author_id` -> `author`, `post_id` -> `post`.
/// If no `_id` suffix, uses column name as-is: `invited_by` -> `invited_by`.
fn forward_method_name(column_name: &str) -> String {
    if let Some(stripped) = column_name.strip_suffix("_id") {
        stripped.to_string()
    } else {
        column_name.to_string()
    }
}

/// Derive the reverse method name from a source table name.
/// Lowercase + `s`: `Post` -> `posts`, `Comment` -> `comments`.
fn reverse_method_name(source_table_ident: &Ident) -> String {
    format!("{}s", source_table_ident.to_string().to_snake_case())
}

/// Generates forward relations (One/OptionalOne from this table to target).
fn generate_forward_relations(
    struct_ident: &Ident,
    vis: &Visibility,
    fk_infos: &[FkInfo],
) -> TokenStream {
    let mut tokens = TokenStream::new();

    for fk in fk_infos {
        let method_name_str = forward_method_name(&fk.source_column);
        let method_name = format_ident!("{}", method_name_str);
        let rel_zst = format_ident!("__Rel_{struct_ident}_{}", to_pascal(&method_name_str));
        let accessor_trait = format_ident!(
            "__QueryAccess_{struct_ident}_{}",
            to_pascal(&method_name_str)
        );
        let rel_accessor_trait = format_ident!(
            "__{struct_ident}_{}_RelAccessor",
            to_pascal(&method_name_str)
        );

        let target_table = &fk.target_table_ident;
        let target_select = format_ident!("Select{}", target_table);
        let source_col = &fk.source_column;
        let target_col = fk.target_column_ident.to_string();

        // Determine cardinality: nullable FK -> OptionalOne, else One
        let card_type = if fk.is_nullable {
            quote!(drizzle::core::relation::OptionalOne)
        } else {
            quote!(drizzle::core::relation::One)
        };

        // Type alias: e.g., `QPostWithAuthor<Rest = ()>`
        let type_alias_ident = format_ident!("{}With{}", struct_ident, to_pascal(&method_name_str));
        let data_type = if fk.is_nullable {
            quote!(Option<drizzle::core::query::QueryRow<#target_select, ()>>)
        } else {
            quote!(drizzle::core::query::QueryRow<#target_select, ()>)
        };

        tokens.extend(quote! {
            #[doc(hidden)]
            #[derive(Debug, Clone, Copy)]
            #vis struct #rel_zst;

            impl drizzle::core::relation::private::Sealed for #rel_zst {}

            impl drizzle::core::relation::RelationDef for #rel_zst {
                type Source = #struct_ident;
                type Target = #target_table;
                type Card = #card_type;
                const NAME: &'static str = #method_name_str;
                fn fk_columns() -> &'static [(&'static str, &'static str)] {
                    &[(#target_col, #source_col)]
                }
            }

            /// Type alias for a `RelEntry` containing this relation's data.
            ///
            /// Use this in function signatures to accept query results with this relation loaded:
            /// ```ignore
            /// fn process(row: &QueryRow<SelectModel, TypeAlias>) { ... }
            /// ```
            ///
            /// The `__Rest` parameter allows composing multiple relations:
            /// ```ignore
            /// fn process(row: &QueryRow<SelectModel, WithPosts<WithAuthor>>) { ... }
            /// ```
            #vis type #type_alias_ident<__Rest = ()> =
                drizzle::core::query::RelEntry<#rel_zst, #data_type, __Rest>;

            // Accessor via extension trait
            #[doc(hidden)]
            #vis trait #rel_accessor_trait {
                fn #method_name<__V: drizzle::core::SQLParam>(&self) -> drizzle::core::query::RelationHandle<__V, #rel_zst>;
            }

            impl #rel_accessor_trait for #struct_ident {
                fn #method_name<__V: drizzle::core::SQLParam>(&self) -> drizzle::core::query::RelationHandle<__V, #rel_zst> {
                    drizzle::core::query::RelationHandle::new()
                }
            }

            // Result accessor trait
            #[doc(hidden)]
            #vis trait #accessor_trait<W> {
                type Data;
                fn #method_name(&self) -> &Self::Data;
            }

            impl<Base, Store, W> #accessor_trait<W> for drizzle::core::query::QueryRow<Base, Store>
            where
                Store: drizzle::core::query::FindRel<#rel_zst, W>,
            {
                type Data = <Store as drizzle::core::query::FindRel<#rel_zst, W>>::Data;
                fn #method_name(&self) -> &Self::Data {
                    self.store.get()
                }
            }
        });
    }

    tokens
}

/// Generates reverse relations (Many from target tables back to this table).
fn generate_reverse_relations(
    struct_ident: &Ident,
    vis: &Visibility,
    fk_infos: &[FkInfo],
) -> TokenStream {
    let mut tokens = TokenStream::new();

    // Count FKs per target table to detect multi-FK situations
    let mut target_fk_counts = std::collections::HashMap::new();
    for fk in fk_infos {
        let target_name = fk.target_table_ident.to_string();
        *target_fk_counts.entry(target_name).or_insert(0usize) += 1;
    }

    // Track which targets already had reverse generated to avoid duplicates
    let mut seen_targets = std::collections::HashSet::new();

    for fk in fk_infos {
        let target_name = fk.target_table_ident.to_string();

        // Skip self-referential FKs
        if fk.target_table_ident == *struct_ident {
            continue;
        }

        // Skip multiple FKs to same target (ambiguous reverse)
        if target_fk_counts[&target_name] > 1 {
            continue;
        }

        // Skip if already generated for this target
        if !seen_targets.insert(target_name.clone()) {
            continue;
        }

        let target_table = &fk.target_table_ident;
        let source_select = format_ident!("Select{}", struct_ident);
        let method_name_str = reverse_method_name(struct_ident);
        let method_name = format_ident!("{}", method_name_str);
        let rel_zst = format_ident!("__Rel_{target_table}_{}", to_pascal(&method_name_str));
        let accessor_trait = format_ident!(
            "__QueryAccess_{target_table}_{}",
            to_pascal(&method_name_str)
        );
        let rel_accessor_trait = format_ident!(
            "__{target_table}_{}_RelAccessor",
            to_pascal(&method_name_str)
        );

        let source_col = &fk.source_column;
        let target_col = fk.target_column_ident.to_string();

        // Type alias: e.g., `QUserWithQPosts<Rest = ()>`
        let type_alias_ident = format_ident!("{}With{}", target_table, to_pascal(&method_name_str));

        tokens.extend(quote! {
            #[doc(hidden)]
            #[derive(Debug, Clone, Copy)]
            #vis struct #rel_zst;

            impl drizzle::core::relation::private::Sealed for #rel_zst {}

            impl drizzle::core::relation::RelationDef for #rel_zst {
                type Source = #target_table;
                type Target = #struct_ident;
                type Card = drizzle::core::relation::Many;
                const NAME: &'static str = #method_name_str;
                fn fk_columns() -> &'static [(&'static str, &'static str)] {
                    &[(#source_col, #target_col)]
                }
            }

            /// Type alias for a `RelEntry` containing this relation's data.
            ///
            /// Use this in function signatures to accept query results with this relation loaded.
            /// The `__Rest` parameter allows composing multiple relations.
            #vis type #type_alias_ident<__Rest = ()> =
                drizzle::core::query::RelEntry<
                    #rel_zst,
                    Vec<drizzle::core::query::QueryRow<#source_select, ()>>,
                    __Rest,
                >;

            #[doc(hidden)]
            #vis trait #rel_accessor_trait {
                fn #method_name<__V: drizzle::core::SQLParam>(&self) -> drizzle::core::query::RelationHandle<__V, #rel_zst>;
            }

            impl #rel_accessor_trait for #target_table {
                fn #method_name<__V: drizzle::core::SQLParam>(&self) -> drizzle::core::query::RelationHandle<__V, #rel_zst> {
                    drizzle::core::query::RelationHandle::new()
                }
            }

            #[doc(hidden)]
            #vis trait #accessor_trait<W> {
                type Data;
                fn #method_name(&self) -> &Self::Data;
            }

            impl<Base, Store, W> #accessor_trait<W> for drizzle::core::query::QueryRow<Base, Store>
            where
                Store: drizzle::core::query::FindRel<#rel_zst, W>,
            {
                type Data = <Store as drizzle::core::query::FindRel<#rel_zst, W>>::Data;
                fn #method_name(&self) -> &Self::Data {
                    self.store.get()
                }
            }
        });
    }

    tokens
}

/// Generates a `FromJsonValue` impl for a model.
///
/// When `nullable_all` is true (partial select), every field is treated as nullable
/// regardless of the original schema — producing `Option<T>` for all fields.
///
/// Uses `deserialize_field()` (zero-copy serde) for standard fields. UUID and enum
/// fields use specialized extraction (parse from string, TryFrom/FromStr).
fn generate_from_json_value_impl(
    model_ident: &Ident,
    fields: &[FieldJsonInfo],
    nullable_all: bool,
) -> TokenStream {
    let field_reads: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let col_name = &f.column_name;
            let is_nullable = nullable_all || f.is_nullable;

            if f.is_uuid {
                return generate_uuid_read(ident, col_name, is_nullable);
            }

            if let Some(storage) = f.enum_storage {
                return generate_enum_read(ident, col_name, &f.base_type, storage, is_nullable);
            }

            generate_serde_read(ident, col_name, is_nullable)
        })
        .collect();

    quote! {
        impl drizzle::core::query::FromJsonValue for #model_ident {
            fn from_json_value(val: &drizzle::core::serde_json::Value) -> ::std::result::Result<Self, drizzle::error::DrizzleError> {
                let obj = val.as_object().ok_or_else(|| {
                    drizzle::error::DrizzleError::Other(
                        ::std::format!("expected JSON object for {}", ::std::stringify!(#model_ident)).into()
                    )
                })?;
                Ok(Self {
                    #(#field_reads),*
                })
            }
        }
    }
}

/// Generates `FromJsonValue` for the full select model.
fn generate_from_json_value(model_ident: &Ident, fields: &[FieldJsonInfo]) -> TokenStream {
    generate_from_json_value_impl(model_ident, fields, false)
}

/// Generates `FromJsonValue` for the partial select model (all fields nullable).
fn generate_partial_from_json_value(model_ident: &Ident, fields: &[FieldJsonInfo]) -> TokenStream {
    generate_from_json_value_impl(model_ident, fields, true)
}

/// Generates a column selector struct and `.columns()` method on the table.
fn generate_column_selector(
    struct_ident: &Ident,
    vis: &Visibility,
    fields: &[FieldJsonInfo],
) -> TokenStream {
    let selector_ident = format_ident!("{}ColumnSelector", struct_ident);
    let accessor_trait_ident = format_ident!("__ColumnsAccessor_{}", struct_ident);

    let builder_methods: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let method_name = &f.ident;
            let col_name = &f.column_name;
            quote! {
                pub fn #method_name(mut self) -> Self {
                    self.selected.push(#col_name);
                    self
                }
            }
        })
        .collect();

    quote! {
        /// Column selector for partial column queries.
        #vis struct #selector_ident {
            selected: ::std::vec::Vec<&'static str>,
        }

        impl #selector_ident {
            #(#builder_methods)*
        }

        impl drizzle::core::query::IntoColumnSelection for #selector_ident {
            fn into_column_names(self) -> ::std::vec::Vec<&'static str> {
                self.selected
            }
        }

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis trait #accessor_trait_ident {
            fn select_columns(&self) -> #selector_ident;
        }

        impl #accessor_trait_ident for #struct_ident {
            fn select_columns(&self) -> #selector_ident {
                #selector_ident { selected: ::std::vec::Vec::new() }
            }
        }
    }
}

// =============================================================================
// Field read helpers (shared by full and partial select)
// =============================================================================

/// Generates a UUID field read (nullable or non-nullable).
fn generate_uuid_read(ident: &Ident, col_name: &str, is_nullable: bool) -> TokenStream {
    if is_nullable {
        quote! {
            #ident: match obj.get(#col_name) {
                Some(drizzle::core::serde_json::Value::String(s)) => Some(s.parse().map_err(|e| drizzle::error::DrizzleError::Other(::std::format!("invalid UUID: {e}").into()))?),
                Some(drizzle::core::serde_json::Value::Null) | None => None,
                _ => None,
            }
        }
    } else {
        quote! {
            #ident: {
                let s = obj.get(#col_name)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| drizzle::error::DrizzleError::Other(::std::format!("missing field '{}'", #col_name).into()))?;
                s.parse().map_err(|e| drizzle::error::DrizzleError::Other(::std::format!("invalid UUID '{}': {e}", #col_name).into()))?
            }
        }
    }
}

/// Generates an enum field read (nullable or non-nullable).
fn generate_enum_read(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    storage: EnumStorage,
    is_nullable: bool,
) -> TokenStream {
    let conversion = enum_json_conversion(base_type, col_name, storage);
    if is_nullable {
        quote! {
            #ident: match obj.get(#col_name) {
                Some(drizzle::core::serde_json::Value::Null) | None => None,
                Some(v) => Some({ #conversion }),
            }
        }
    } else {
        quote! {
            #ident: {
                let v = obj.get(#col_name)
                    .ok_or_else(|| drizzle::error::DrizzleError::Other(::std::format!("missing field '{}'", #col_name).into()))?;
                #conversion
            }
        }
    }
}

/// Generates a field read via serde deserialization.
fn generate_serde_read(ident: &Ident, col_name: &str, is_nullable: bool) -> TokenStream {
    if is_nullable {
        quote! {
            #ident: match obj.get(#col_name) {
                Some(drizzle::core::serde_json::Value::Null) | None => None,
                Some(v) => Some(drizzle::core::query::deserialize_field(v, #col_name)?),
            }
        }
    } else {
        quote! {
            #ident: {
                let v = obj.get(#col_name)
                    .ok_or_else(|| drizzle::error::DrizzleError::Other(::std::format!("missing field '{}'", #col_name).into()))?;
                drizzle::core::query::deserialize_field(v, #col_name)?
            }
        }
    }
}

/// Generates the conversion expression for an enum field from a JSON value `v`.
fn enum_json_conversion(
    field_type: &syn::Type,
    col_name: &str,
    storage: EnumStorage,
) -> TokenStream {
    match storage {
        EnumStorage::Integer => {
            quote! {
                {
                    let n = v.as_i64().ok_or_else(|| drizzle::error::DrizzleError::Other(
                        ::std::format!("enum field '{}': expected integer", #col_name).into()
                    ))?;
                    <#field_type as ::std::convert::TryFrom<i64>>::try_from(n)?
                }
            }
        }
        EnumStorage::Text => {
            quote! {
                {
                    let s = v.as_str().ok_or_else(|| drizzle::error::DrizzleError::Other(
                        ::std::format!("enum field '{}': expected string", #col_name).into()
                    ))?;
                    <#field_type as ::std::str::FromStr>::from_str(s)
                        .map_err(|e| drizzle::error::DrizzleError::Other(
                            ::std::format!("enum field '{}': {e}", #col_name).into()
                        ))?
                }
            }
        }
    }
}

/// Convert a snake_case string to PascalCase.
fn to_pascal(s: &str) -> String {
    use heck::ToUpperCamelCase;
    s.to_upper_camel_case()
}
