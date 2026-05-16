//! Shared query API code generation for both `SQLite` and `PostgreSQL`.
//!
//! Generates relation ZSTs, `RelationDef` impls, inherent accessor methods,
//! result accessor traits, type aliases, JSON decoder impls, and column
//! selectors from FK declarations.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Visibility;

/// FK info extracted from field declarations.
pub struct FkInfo {
    /// Source column name (e.g., "`author_id`").
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
pub enum EnumStorage {
    /// Stored as INTEGER — deserialize via `TryFrom<i64>`.
    Integer,
    /// Stored as TEXT — deserialize via `FromStr`.
    Text,
}

/// How a field should be read from JSON. These storage kinds are mutually
/// exclusive and each takes a distinct decode path.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldStorageKind {
    /// Plain JSON-native value (number, string, array, object).
    Plain,
    /// UUID parsed from a string.
    Uuid,
    /// Boolean. `SQLite` stores booleans as integers (0/1) which appear as JSON
    /// numbers inside `json_object()`.
    Bool,
    /// Raw blob (`Vec<u8>`).
    Blob,
}

/// Info about a field for generating JSON decoders.
pub struct FieldJsonInfo {
    /// The field ident (e.g., `id`).
    pub ident: Ident,
    /// The column name in SQL (e.g., "id").
    pub column_name: String,
    /// Whether the field is nullable.
    pub is_nullable: bool,
    /// Whether the field is a JSON/JSONB field (orthogonal to `storage`; a blob
    /// field may carry JSON content).
    pub is_json: bool,
    /// How this field is stored and therefore how it should be read back.
    pub storage: FieldStorageKind,
    /// If the field is an enum, how it is stored in the database.
    pub enum_storage: Option<EnumStorage>,
    /// The unwrapped base type (e.g., `i32` even if the field is `Option<i32>`).
    pub base_type: syn::Type,
    /// The generated select model field type.
    pub select_type: TokenStream,
    /// The generated partial select model field type.
    pub partial_select_type: TokenStream,
}

/// Generates all query API code for a table.
///
/// Returns a `TokenStream` containing:
/// - `QueryTable` impl for the table ZST
/// - Forward relation items (ZST, `RelationDef` impl, accessor method, result accessor trait, type alias)
/// - Reverse relation items (ZST, `RelationDef` impl, accessor method, result accessor trait, type alias)
/// - JSON decoder impl for the select model
/// - JSON decoder impl for the partial select model
/// - Column selector struct and `.columns()` method
#[allow(clippy::too_many_arguments)]
pub fn generate_query_api(
    struct_ident: &Ident,
    struct_vis: &Visibility,
    table_name: &str,
    select_model_ident: &Ident,
    partial_select_model_ident: &Ident,
    fk_infos: &[FkInfo],
    field_json_infos: &[FieldJsonInfo],
    column_names: &[String],
) -> TokenStream {
    let mut tokens = TokenStream::new();

    // Collect blob column names (UUID and Vec<u8> types — stored as BLOB in SQLite).
    let blob_column_names: Vec<&str> = field_json_infos
        .iter()
        .filter(|f| matches!(f.storage, FieldStorageKind::Uuid | FieldStorageKind::Blob))
        .map(|f| f.column_name.as_str())
        .collect();

    // 1. Generate QueryTable impl (table name, column names, select model, partial select model)
    tokens.extend(generate_query_table(
        struct_ident,
        select_model_ident,
        partial_select_model_ident,
        table_name,
        column_names,
        &blob_column_names,
    ));

    // 1b. Generate QueryRow type alias for cleaner function signatures
    let query_row_alias = format_ident!("{}QueryRow", struct_ident);
    tokens.extend(quote! {
        /// Type alias for a query result row from this table.
        ///
        /// Use `S` to specify loaded relations:
        /// ```rust
        /// # type UsersWithPosts = ();
        /// # type UsersQueryRow<T> = T;
        /// fn process(rows: &[UsersQueryRow<UsersWithPosts>]) {
        ///     let _ = rows;
        /// }
        /// ```
        #struct_vis type #query_row_alias<S = ()> = drizzle::core::query::QueryRow<#select_model_ident, S>;
    });

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

    // 4. Generate many-to-many relations (junction table detection)
    tokens.extend(generate_many_to_many_relations(
        struct_ident,
        struct_vis,
        table_name,
        fk_infos,
    ));

    // 5. Generate JSON decoder for the select model
    tokens.extend(generate_json_decoder(
        select_model_ident,
        field_json_infos,
        false,
    ));

    // 6. Generate JSON decoder for the partial select model (all fields optional)
    tokens.extend(generate_json_decoder(
        partial_select_model_ident,
        field_json_infos,
        true,
    ));

    // 7. Generate column selector struct and `.columns()` method
    tokens.extend(generate_column_selector(
        struct_ident,
        struct_vis,
        field_json_infos,
    ));

    tokens
}

/// Generates `QueryTable` impl for the table ZST.
fn generate_query_table(
    struct_ident: &Ident,
    select_model_ident: &Ident,
    partial_select_model_ident: &Ident,
    table_name: &str,
    column_names: &[String],
    blob_column_names: &[&str],
) -> TokenStream {
    let column_name_literals: Vec<&str> = column_names
        .iter()
        .map(std::string::String::as_str)
        .collect();

    let blob_const = if blob_column_names.is_empty() {
        // Use default (empty slice) — no override needed
        quote! {}
    } else {
        quote! {
            const BLOB_COLUMNS: &'static [&'static str] = &[#(#blob_column_names),*];
        }
    };

    quote! {
        impl drizzle::core::query::QueryTable for #struct_ident {
            type Select = #select_model_ident;
            type PartialSelect = #partial_select_model_ident;
            const TABLE_NAME: &'static str = #table_name;
            const COLUMN_NAMES: &'static [&'static str] = &[#(#column_name_literals),*];
            #blob_const
        }
    }
}

/// Derive the forward method name from a column name.
/// Strips `_id` suffix: `author_id` -> `author`, `post_id` -> `post`.
/// If no `_id` suffix, uses column name as-is: `invited_by` -> `invited_by`.
fn forward_method_name(column_name: &str) -> String {
    column_name.strip_suffix("_id").map_or_else(
        || column_name.to_string(),
        |stripped| {
            if stripped.is_empty() {
                column_name.to_string()
            } else {
                stripped.to_string()
            }
        },
    )
}

/// Derive the reverse method name from a source table name.
/// Lowercase + pluralize: `Post` -> `posts`, `Category` -> `categories`.
fn reverse_method_name(source_table_ident: &Ident) -> String {
    pluralize(&source_table_ident.to_string().to_snake_case())
}

/// Pluralize an English word using the `pluralizer` crate.
fn pluralize(s: &str) -> String {
    pluralizer::pluralize(s, 2, false)
}

/// Parameters needed to emit one relation (ZST + `RelationDef` + accessor
/// method + result accessor trait + type alias) regardless of cardinality.
///
/// Forward, reverse, and many-to-many generation all produce the same
/// skeleton; only the source/target types, cardinality, FK columns body,
/// optional junction body, accessor receiver, and the type alias's data
/// slot differ. `RelEmitter::emit` is the sole place that knows the skeleton.
struct RelEmitter<'a> {
    /// `pub` / `pub(crate)` from the host struct.
    vis: &'a Visibility,
    /// `__Rel_X_Y` — the ZST that implements `RelationDef`.
    rel_zst: Ident,
    /// `QueryXY` — the result accessor trait.
    accessor_trait: Ident,
    /// `XWithY` — public type alias for the loaded-relation row.
    type_alias: Ident,
    /// Method ident on both the accessor receiver and the accessor trait.
    method: Ident,
    /// `RelationDef::NAME` — the method name string for runtime use.
    method_str: String,
    /// `RelationDef::Source` type tokens.
    source: TokenStream,
    /// `RelationDef::Target` type tokens.
    target: TokenStream,
    /// `RelationDef::Card` (One / OptionalOne / Many).
    card: TokenStream,
    /// Body of `fn fk_columns()` — usually `&[(...)]` or `&[]` for M2M.
    fk_columns_body: TokenStream,
    /// `Some(body)` to emit `fn junction()`, `None` to omit it.
    junction_body: Option<TokenStream>,
    /// Receiver of the accessor method's inherent impl
    /// (e.g. `__XForwardRels`, the target table ZST, the source table ZST).
    accessor_receiver: TokenStream,
    /// Data slot of `RelEntry` in the type alias
    /// (e.g. `QueryRow<...>`, `Option<QueryRow<...>>`, `Vec<QueryRow<...>>`).
    data_type: TokenStream,
}

impl RelEmitter<'_> {
    fn emit(&self) -> TokenStream {
        let RelEmitter {
            vis,
            rel_zst,
            accessor_trait,
            type_alias,
            method,
            method_str,
            source,
            target,
            card,
            fk_columns_body,
            junction_body,
            accessor_receiver,
            data_type,
        } = self;

        let junction_fn = junction_body.as_ref().map(|body| {
            quote! {
                fn junction() -> Option<drizzle::core::relation::JunctionMeta> { #body }
            }
        });

        quote! {
            #[doc(hidden)]
            #[derive(Debug, Clone, Copy)]
            #[allow(non_camel_case_types)]
            #vis struct #rel_zst;

            impl drizzle::core::relation::private::Sealed for #rel_zst {}

            impl drizzle::core::relation::RelationDef for #rel_zst {
                type Source = #source;
                type Target = #target;
                type Card = #card;
                const NAME: &'static str = #method_str;
                fn fk_columns() -> &'static [(&'static str, &'static str)] {
                    #fk_columns_body
                }
                #junction_fn
            }

            /// Type alias for query results with this relation loaded.
            ///
            /// Nest `Rest` to compose multiple relations.
            #vis type #type_alias<Rest = ()> =
                drizzle::core::query::RelEntry<#rel_zst, #data_type, Rest>;

            impl #accessor_receiver {
                #vis fn #method<__V: drizzle::core::SQLParam>(&self) -> drizzle::core::query::RelationHandle<__V, #rel_zst> {
                    drizzle::core::query::RelationHandle::new()
                }
            }

            #vis trait #accessor_trait<W> {
                type Data;
                fn #method(&self) -> &Self::Data;
            }

            impl<Base, Store, W> #accessor_trait<W> for drizzle::core::query::QueryRow<Base, Store>
            where
                Store: drizzle::core::query::FindRel<#rel_zst, W>,
            {
                type Data = <Store as drizzle::core::query::FindRel<#rel_zst, W>>::Data;
                fn #method(&self) -> &Self::Data {
                    self.store.get()
                }
            }
        }
    }
}

/// Generates forward relations (One/OptionalOne from this table to target).
///
/// Forward relation accessor methods live on a hidden `__{Table}ForwardRels`
/// struct, reached via `Deref` on the table ZST. This avoids name collisions
/// with the per-column associated constants that the table macro generates
/// (e.g., `const invited_by: InvitedByColumn`), while still allowing
/// `table.relation()` calls without any trait import.
fn generate_forward_relations(
    struct_ident: &Ident,
    vis: &Visibility,
    fk_infos: &[FkInfo],
) -> TokenStream {
    let mut tokens = TokenStream::new();

    if fk_infos.is_empty() {
        return tokens;
    }

    // Hidden struct that holds all forward relation accessor methods.
    // The table ZST derefs to this, so `table.relation()` resolves here.
    let rels_struct = format_ident!("__{struct_ident}ForwardRels");

    tokens.extend(quote! {
        #[doc(hidden)]
        #vis struct #rels_struct;

        impl ::std::ops::Deref for #struct_ident {
            type Target = #rels_struct;
            fn deref(&self) -> &#rels_struct {
                &#rels_struct
            }
        }
    });

    for fk in fk_infos {
        let method_name_str = forward_method_name(&fk.source_column);
        let target_table = &fk.target_table_ident;
        let target_select = format_ident!("Select{}", target_table);
        let source_col = &fk.source_column;
        let target_col = fk.target_column_ident.to_string();
        let method_pascal = to_pascal(&method_name_str);

        // Nullable FK -> OptionalOne + Option<...> data slot, else One.
        let (card, data_type) = if fk.is_nullable {
            (
                quote!(drizzle::core::relation::OptionalOne),
                quote!(Option<drizzle::core::query::QueryRow<#target_select, ()>>),
            )
        } else {
            (
                quote!(drizzle::core::relation::One),
                quote!(drizzle::core::query::QueryRow<#target_select, ()>),
            )
        };

        tokens.extend(
            RelEmitter {
                vis,
                rel_zst: format_ident!("__Rel_{struct_ident}_{method_pascal}"),
                accessor_trait: format_ident!("Query{struct_ident}{method_pascal}"),
                type_alias: format_ident!("{struct_ident}With{method_pascal}"),
                method: format_ident!("{method_name_str}"),
                method_str: method_name_str.clone(),
                source: quote!(#struct_ident),
                target: quote!(#target_table),
                card,
                fk_columns_body: quote!(&[(#target_col, #source_col)]),
                junction_body: None,
                accessor_receiver: quote!(#rels_struct),
                data_type,
            }
            .emit(),
        );
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
        let method_pascal = to_pascal(&method_name_str);
        let source_col = &fk.source_column;
        let target_col = fk.target_column_ident.to_string();

        tokens.extend(
            RelEmitter {
                vis,
                rel_zst: format_ident!("__Rel_{target_table}_{method_pascal}"),
                accessor_trait: format_ident!("Query{target_table}{method_pascal}"),
                type_alias: format_ident!("{target_table}With{method_pascal}"),
                method: format_ident!("{method_name_str}"),
                method_str: method_name_str.clone(),
                source: quote!(#target_table),
                target: quote!(#struct_ident),
                card: quote!(drizzle::core::relation::Many),
                fk_columns_body: quote!(&[(#source_col, #target_col)]),
                junction_body: None,
                accessor_receiver: quote!(#target_table),
                data_type: quote!(Vec<drizzle::core::query::QueryRow<#source_select, ()>>),
            }
            .emit(),
        );
    }

    tokens
}

/// Generates many-to-many relations through a junction table.
///
/// A junction table is detected when:
/// - Exactly 2 FK columns exist
/// - They target 2 different tables
/// - Neither target is the junction table itself
///
/// For each direction, generates a relation from one target to the other
/// through the junction, with `Card = Many` and `fn junction()`.
fn generate_many_to_many_relations(
    struct_ident: &Ident,
    vis: &Visibility,
    table_name: &str,
    fk_infos: &[FkInfo],
) -> TokenStream {
    // Junction detection: exactly 2 FKs to 2 different external tables
    if fk_infos.len() != 2 {
        return TokenStream::new();
    }
    let (fk_a, fk_b) = (&fk_infos[0], &fk_infos[1]);
    if fk_a.target_table_ident == fk_b.target_table_ident {
        return TokenStream::new();
    }
    if fk_a.target_table_ident == *struct_ident || fk_b.target_table_ident == *struct_ident {
        return TokenStream::new();
    }

    let mut tokens = TokenStream::new();

    let junction_pascal = to_pascal(&struct_ident.to_string());

    // Generate both directions: A→B and B→A through the junction
    for (source_fk, target_fk) in [(fk_a, fk_b), (fk_b, fk_a)] {
        let source_table = &source_fk.target_table_ident;
        let target_table = &target_fk.target_table_ident;
        let target_select = format_ident!("Select{}", target_table);

        let method_name_str = reverse_method_name(target_table);
        let method_pascal = to_pascal(&method_name_str);

        let source_col_name = &source_fk.source_column;
        let source_target_col = source_fk.target_column_ident.to_string();
        let target_col_name = &target_fk.source_column;
        let target_target_col = target_fk.target_column_ident.to_string();

        tokens.extend(
            RelEmitter {
                vis,
                // Include junction table name to avoid collisions with reverse relations
                rel_zst: format_ident!("__Rel_{source_table}_Via{junction_pascal}_{method_pascal}"),
                accessor_trait: format_ident!(
                    "Query{source_table}Via{junction_pascal}{method_pascal}"
                ),
                type_alias: format_ident!("{source_table}Via{junction_pascal}With{method_pascal}"),
                method: format_ident!("{method_name_str}"),
                method_str: method_name_str.clone(),
                source: quote!(#source_table),
                target: quote!(#target_table),
                card: quote!(drizzle::core::relation::Many),
                fk_columns_body: quote!(&[]),
                junction_body: Some(quote! {
                    Some(drizzle::core::relation::JunctionMeta {
                        table_name: #table_name,
                        source_fk: &[(#source_col_name, #source_target_col)],
                        target_fk: &[(#target_col_name, #target_target_col)],
                    })
                }),
                accessor_receiver: quote!(#source_table),
                data_type: quote!(Vec<drizzle::core::query::QueryRow<#target_select, ()>>),
            }
            .emit(),
        );
    }

    tokens
}

/// Generates JSON decoder impls for a model.
///
/// When `nullable_all` is true, every field is treated as nullable regardless
/// of the original schema.
fn generate_json_decoder(
    model_ident: &Ident,
    fields: &[FieldJsonInfo],
    nullable_all: bool,
) -> TokenStream {
    let state_ident = format_ident!("__{model_ident}JsonState");

    let state_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = if nullable_all {
                &f.partial_select_type
            } else {
                &f.select_type
            };

            quote! { #ident: ::std::option::Option<#ty> }
        })
        .collect();

    let init_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! { #ident: ::std::option::Option::None }
        })
        .collect();

    let field_matches: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let col_name = &f.column_name;
            let is_nullable = nullable_all || f.is_nullable;

            if let Some(storage) = f.enum_storage {
                return generate_enum_decode(ident, col_name, &f.base_type, storage, is_nullable);
            }

            match f.storage {
                FieldStorageKind::Uuid => {
                    generate_uuid_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::Bool => generate_bool_decode(ident, col_name, is_nullable),
                FieldStorageKind::Blob => {
                    generate_blob_decode(ident, col_name, &f.base_type, is_nullable, f.is_json)
                }
                FieldStorageKind::Plain => {
                    let ty = if nullable_all {
                        &f.partial_select_type
                    } else {
                        &f.select_type
                    };
                    generate_plain_decode(ident, col_name, ty)
                }
            }
        })
        .collect();

    let finish_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let col_name = &f.column_name;
            let is_nullable = nullable_all || f.is_nullable;
            if is_nullable {
                quote! {
                    #ident: state.#ident.unwrap_or(::std::option::Option::None)
                }
            } else {
                quote! {
                    #ident: state.#ident.ok_or_else(|| <__E as drizzle::core::serde::de::Error>::missing_field(#col_name))?
                }
            }
        })
        .collect();

    quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #state_ident {
            #(#state_fields,)*
        }

        impl<'de> drizzle::core::query::JsonObjectDecoder<'de> for #model_ident {
            type State = #state_ident;

            fn begin() -> Self::State {
                #state_ident {
                    #(#init_fields,)*
                }
            }

            fn decode_field<__A>(
                state: &mut Self::State,
                key: &str,
                map: &mut __A,
            ) -> ::std::result::Result<bool, __A::Error>
            where
                __A: drizzle::core::serde::de::MapAccess<'de>,
            {
                match key {
                    #(#field_matches,)*
                    _ => ::std::result::Result::Ok(false),
                }
            }

            fn finish<__E>(state: Self::State) -> ::std::result::Result<Self, __E>
            where
                __E: drizzle::core::serde::de::Error,
            {
                ::std::result::Result::Ok(Self {
                    #(#finish_fields,)*
                })
            }
        }

        impl<'de> drizzle::core::serde::Deserialize<'de> for #model_ident {
            fn deserialize<__D>(deserializer: __D) -> ::std::result::Result<Self, __D::Error>
            where
                __D: drizzle::core::serde::Deserializer<'de>,
            {
                struct __Visitor;

                impl<'de> drizzle::core::serde::de::Visitor<'de> for __Visitor {
                    type Value = #model_ident;

                    fn expecting(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        f.write_str("a row JSON object")
                    }

                    fn visit_map<__A>(self, mut map: __A) -> ::std::result::Result<Self::Value, __A::Error>
                    where
                        __A: drizzle::core::serde::de::MapAccess<'de>,
                    {
                        let mut state = <#model_ident as drizzle::core::query::JsonObjectDecoder<'de>>::begin();
                        while let ::std::option::Option::Some(key) = map.next_key::<::std::borrow::Cow<'de, str>>()? {
                            if <#model_ident as drizzle::core::query::JsonObjectDecoder<'de>>::decode_field(
                                &mut state,
                                key.as_ref(),
                                &mut map,
                            )? {
                                continue;
                            }
                            map.next_value::<drizzle::core::serde::de::IgnoredAny>()?;
                        }
                        <#model_ident as drizzle::core::query::JsonObjectDecoder<'de>>::finish(state)
                    }
                }

                deserializer.deserialize_map(__Visitor)
            }
        }
    }
}

/// Generates a column selector struct and `.columns()` method on the table.
fn generate_column_selector(
    struct_ident: &Ident,
    vis: &Visibility,
    fields: &[FieldJsonInfo],
) -> TokenStream {
    let selector_ident = format_ident!("{}ColumnSelector", struct_ident);

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
        #[doc(hidden)]
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

        // Column selector — inherent method on the table ZST
        impl #struct_ident {
            #vis fn columns(&self) -> #selector_ident {
                #selector_ident { selected: ::std::vec::Vec::new() }
            }
        }
    }
}

// =============================================================================
// Field decode helpers (shared by full and partial select)
// =============================================================================

fn generate_plain_decode(ident: &Ident, col_name: &str, field_type: &TokenStream) -> TokenStream {
    quote! {
        #col_name => {
            state.#ident = ::std::option::Option::Some(map.next_value::<#field_type>()?);
            ::std::result::Result::Ok(true)
        }
    }
}

fn generate_uuid_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    if is_nullable {
        quote! {
            #col_name => {
                let raw = map.next_value::<::std::option::Option<::std::string::String>>()?;
                state.#ident = ::std::option::Option::Some(raw
                    .map(|s| {
                        s.parse::<#base_type>().map_err(|e| {
                            <__A::Error as drizzle::core::serde::de::Error>::custom(
                                ::std::format!("field '{}': invalid UUID: {e}", #col_name)
                            )
                        })
                    })
                    .transpose()?);
                ::std::result::Result::Ok(true)
            }
        }
    } else {
        quote! {
            #col_name => {
                let raw = map.next_value::<::std::string::String>()?;
                state.#ident = ::std::option::Option::Some(raw.parse::<#base_type>().map_err(|e| {
                    <__A::Error as drizzle::core::serde::de::Error>::custom(
                        ::std::format!("field '{}': invalid UUID: {e}", #col_name)
                    )
                })?);
                ::std::result::Result::Ok(true)
            }
        }
    }
}

fn generate_bool_decode(ident: &Ident, col_name: &str, is_nullable: bool) -> TokenStream {
    if is_nullable {
        quote! {
            #col_name => {
                state.#ident = ::std::option::Option::Some(
                    map.next_value::<drizzle::core::query::JsonOptionalBool>()?.0
                );
                ::std::result::Result::Ok(true)
            }
        }
    } else {
        quote! {
            #col_name => {
                state.#ident = ::std::option::Option::Some(
                    map.next_value::<drizzle::core::query::JsonBool>()?.0
                );
                ::std::result::Result::Ok(true)
            }
        }
    }
}

fn generate_blob_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
    is_json: bool,
) -> TokenStream {
    let convert = if is_json {
        quote! {
            drizzle::core::serde_json::from_slice::<#base_type>(&bytes).map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': invalid JSON blob: {e}", #col_name)
                )
            })?
        }
    } else {
        quote! {
            <#base_type as drizzle::sqlite::traits::FromSQLiteValue>::from_sqlite_blob(&bytes)
                .map_err(|e| {
                    <__A::Error as drizzle::core::serde::de::Error>::custom(
                        ::std::format!("field '{}': {e}", #col_name)
                    )
                })?
        }
    };

    let decode = quote! {
        if s.len() % 2 != 0 {
            return ::std::result::Result::Err(
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': odd-length hex string", #col_name)
                )
            );
        }
        let bytes = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<::std::result::Result<::std::vec::Vec<u8>, _>>()
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': invalid hex: {e}", #col_name)
                )
            })?;
        #convert
    };

    if is_nullable {
        quote! {
            #col_name => {
                let raw = map.next_value::<::std::option::Option<::std::string::String>>()?;
                state.#ident = ::std::option::Option::Some(match raw {
                    ::std::option::Option::Some(s) => ::std::option::Option::Some({ #decode }),
                    ::std::option::Option::None => ::std::option::Option::None,
                });
                ::std::result::Result::Ok(true)
            }
        }
    } else {
        quote! {
            #col_name => {
                let s = map.next_value::<::std::string::String>()?;
                state.#ident = ::std::option::Option::Some({ #decode });
                ::std::result::Result::Ok(true)
            }
        }
    }
}

fn generate_enum_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    storage: EnumStorage,
    is_nullable: bool,
) -> TokenStream {
    let decode_some = enum_json_decode(base_type, col_name, storage);
    if is_nullable {
        let raw_ty = enum_raw_type(storage);
        quote! {
            #col_name => {
                let raw = map.next_value::<::std::option::Option<#raw_ty>>()?;
                state.#ident = ::std::option::Option::Some(match raw {
                    ::std::option::Option::Some(raw) => ::std::option::Option::Some({ #decode_some }),
                    ::std::option::Option::None => ::std::option::Option::None,
                });
                ::std::result::Result::Ok(true)
            }
        }
    } else {
        let raw_ty = enum_raw_type(storage);
        quote! {
            #col_name => {
                let raw = map.next_value::<#raw_ty>()?;
                state.#ident = ::std::option::Option::Some({ #decode_some });
                ::std::result::Result::Ok(true)
            }
        }
    }
}

fn enum_raw_type(storage: EnumStorage) -> TokenStream {
    match storage {
        EnumStorage::Integer => quote!(i64),
        EnumStorage::Text => quote!(::std::string::String),
    }
}

fn enum_json_decode(field_type: &syn::Type, col_name: &str, storage: EnumStorage) -> TokenStream {
    match storage {
        EnumStorage::Integer => quote! {
            <#field_type as ::std::convert::TryFrom<i64>>::try_from(raw)
                .map_err(|_| {
                    <__A::Error as drizzle::core::serde::de::Error>::custom(
                        ::std::format!("enum field '{}': invalid integer value {raw}", #col_name)
                    )
                })?
        },
        EnumStorage::Text => quote! {
            <#field_type as ::std::str::FromStr>::from_str(raw.as_str())
                .map_err(|e| {
                    <__A::Error as drizzle::core::serde::de::Error>::custom(
                        ::std::format!("enum field '{}': {e}", #col_name)
                    )
                })?
        },
    }
}

/// Convert a `snake_case` string to `PascalCase`.
fn to_pascal(s: &str) -> String {
    s.to_upper_camel_case()
}
