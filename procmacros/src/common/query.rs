//! Shared relational-query API code generation for all SQL dialects.
//!
//! Generates the `QueryTable` impl, the relations (see [`super::relations`]),
//! JSON decoder impls, and column selectors.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Visibility;

pub use super::relations::TableKeys;
use super::relations::generate_relations;

/// How an enum field is stored in a dialect's JSON projection.
///
/// PostgreSQL enums and integer-backed enum columns use this direct path.
/// SQLite codec-owned columns use `FieldStorageKind::SQLiteColumn` instead.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum EnumStorage {
    /// Stored as INTEGER — deserialize via `TryFrom<i64>`.
    Integer,
    /// Stored as TEXT — deserialize via `FromStr`.
    Text,
    /// PostgreSQL enum whose storage is owned by its derive. Native enums are
    /// projected as strings; repr enums are projected as numbers.
    Postgres,
}

/// How a field should be read from JSON. These storage kinds are mutually
/// exclusive and each takes a distinct decode path.
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FieldStorageKind {
    /// Plain JSON-native value (number, string, array, object).
    Plain,
    /// UUID parsed from a dialect-native JSON string.
    Uuid,
    /// UUID read from SQLite's tagged JSON projection.
    SQLiteUuid,
    /// Boolean. `SQLite` stores booleans as integers (0/1) which appear as JSON
    /// numbers inside `json_object()`.
    Bool,
    /// Raw blob (`Vec<u8>`).
    Blob,
    /// Binary MySQL value read from the tagged hexadecimal JSON projection.
    MySQLBlob,
    /// MySQL text decoded through the same checked codec as a wire row.
    MySQLText,
    /// A MySQL column whose projection and decoding are owned by
    /// `DrizzleMySQLColumn` and its SQL type marker.
    MySQLColumn,
    /// A SQLite column whose storage and decoding are owned by
    /// `DrizzleSQLiteColumn`.
    SQLiteColumn,
    /// A SQLite JSON column. SQLite stores the document as TEXT, which a
    /// relational projection embeds as a JSON string; the decoder parses it.
    SQLiteJson,
    /// A MySQL JSON column whose payload decodes through the
    /// `drizzle::core::Json<T>` column codec.
    MySQLJson,
}

/// SQL normalization applied before a field enters a relational JSON object.
#[derive(Clone)]
pub enum FieldProjectionKind {
    /// Use the SQL value as-is.
    Native,
    /// Preserve bytes in a tagged hexadecimal object.
    #[cfg(feature = "mysql")]
    TaggedHex,
    /// Cast to text before JSON construction.
    #[cfg(feature = "mysql")]
    Text,
    /// Cast to an unsigned integer before JSON construction.
    #[cfg(feature = "mysql")]
    Unsigned,
    /// Use projection metadata from a custom Rust type's MySQL SQL marker.
    #[cfg(feature = "mysql")]
    MySQLColumn(Box<syn::Type>),
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
    /// How SQL must normalize this field before JSON construction.
    #[cfg_attr(not(feature = "mysql"), allow(dead_code))]
    pub projection: FieldProjectionKind,
    /// Direct enum storage used by dialects without a SQLite column codec.
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
/// - Forward relation items (ZST, `RelationDef` impl, builder accessor, `*With*` row struct)
/// - Reverse relation items (ZST, `RelationDef` impl, builder accessor, `*With*` row struct)
/// - JSON decoder impl for the select model
/// - JSON decoder impl for the partial select model
/// - Column selector struct and `.columns()` method
#[allow(clippy::too_many_arguments)]
pub fn generate_query_api(
    struct_ident: &Ident,
    struct_vis: &Visibility,
    table_schema: Option<&str>,
    table_name: &str,
    select_model_ident: &Ident,
    partial_select_model_ident: &Ident,
    keys: &TableKeys,
    field_json_infos: &[FieldJsonInfo],
) -> TokenStream {
    let mut tokens = TokenStream::new();

    // Collect blob column names (UUID and Vec<u8> types — stored as BLOB in SQLite).
    let blob_column_names: Vec<&str> = field_json_infos
        .iter()
        .filter(|f| {
            matches!(
                f.storage,
                FieldStorageKind::SQLiteUuid
                    | FieldStorageKind::Blob
                    | FieldStorageKind::SQLiteColumn
            )
        })
        .map(|f| f.column_name.as_str())
        .collect();

    // 1. Generate QueryTable impl (table name, column names, select model, partial select model)
    tokens.extend(generate_query_table(
        struct_ident,
        select_model_ident,
        partial_select_model_ident,
        QueryTableMetadata {
            schema: table_schema,
            name: table_name,
            columns: keys.columns(),
            blob_columns: &blob_column_names,
            fields: field_json_infos,
        },
    ));

    // 1b. Type alias for a query result row with no relations loaded.
    let query_row_alias = format_ident!("{}QueryRow", struct_ident);
    tokens.extend(quote! {
        /// Type alias for a query result row from this table with no relations loaded.
        ///
        /// Relation-loaded results use the generated `*With*` structs instead
        /// (for example `UsersWithPosts`).
        #struct_vis type #query_row_alias = #select_model_ident;
    });

    // 2. Relations: forward and reverse per foreign key, many-to-many through
    //    a link table.
    tokens.extend(generate_relations(struct_ident, struct_vis, keys));

    // 3. Generate JSON decoder for the select model
    tokens.extend(generate_json_decoder(
        select_model_ident,
        field_json_infos,
        false,
    ));

    // 4. Generate JSON decoder for the partial select model (all fields optional)
    tokens.extend(generate_json_decoder(
        partial_select_model_ident,
        field_json_infos,
        true,
    ));

    // 5. Generate column selector struct and `.columns()` method
    tokens.extend(generate_column_selector(
        struct_ident,
        struct_vis,
        field_json_infos,
    ));

    tokens
}

/// Generates `QueryTable` impl for the table ZST.
struct QueryTableMetadata<'a> {
    schema: Option<&'a str>,
    name: &'a str,
    columns: &'a [String],
    blob_columns: &'a [&'a str],
    fields: &'a [FieldJsonInfo],
}

fn generate_query_table(
    struct_ident: &Ident,
    select_model_ident: &Ident,
    partial_select_model_ident: &Ident,
    metadata: QueryTableMetadata<'_>,
) -> TokenStream {
    let table_schema = metadata.schema.map_or_else(
        || quote!(::core::option::Option::None),
        |schema| quote!(::core::option::Option::Some(#schema)),
    );
    let table_name = metadata.name;
    let column_name_literals: Vec<&str> = metadata
        .columns
        .iter()
        .map(std::string::String::as_str)
        .collect();
    let blob_column_names = metadata.blob_columns;

    let blob_const = if blob_column_names.is_empty() {
        // Use default (empty slice) — no override needed
        quote! {}
    } else {
        quote! {
            const BLOB_COLUMNS: &'static [&'static str] = &[#(#blob_column_names),*];
        }
    };

    #[cfg(feature = "mysql")]
    let projections: Vec<TokenStream> = metadata
        .fields
        .iter()
        .filter_map(|field| {
            let column = field.column_name.as_str();
            let kind = match &field.projection {
                FieldProjectionKind::Native => return None,
                FieldProjectionKind::TaggedHex => {
                    quote!(drizzle::core::query::JsonProjectionKind::TaggedHex)
                }
                FieldProjectionKind::Text => quote!(drizzle::core::query::JsonProjectionKind::Text),
                FieldProjectionKind::Unsigned => {
                    quote!(drizzle::core::query::JsonProjectionKind::Unsigned)
                }
                FieldProjectionKind::MySQLColumn(base_type) => quote!(
                    <<#base_type as drizzle::mysql::traits::DrizzleMySQLColumn>::SQLType
                        as drizzle::mysql::traits::MySQLColumnType>::JSON_PROJECTION
                ),
            };
            Some(quote! {
                drizzle::core::query::JsonColumnProjection {
                    column: #column,
                    kind: #kind,
                }
            })
        })
        .collect();
    #[cfg(not(feature = "mysql"))]
    let projections = {
        let _ = metadata.fields;
        Vec::<TokenStream>::new()
    };

    quote! {
        impl drizzle::core::query::QueryTable for #struct_ident {
            type Select = #select_model_ident;
            type PartialSelect = #partial_select_model_ident;
            const TABLE_NAME: &'static str = #table_name;
            const TABLE_SCHEMA: ::core::option::Option<&'static str> = #table_schema;
            const COLUMN_NAMES: &'static [&'static str] = &[#(#column_name_literals),*];
            #blob_const
            const JSON_PROJECTIONS: &'static [drizzle::core::query::JsonColumnProjection] = &[
                #(#projections),*
            ];
        }
    }
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
                FieldStorageKind::SQLiteUuid => {
                    generate_sqlite_uuid_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::Bool => generate_bool_decode(ident, col_name, is_nullable),
                FieldStorageKind::Blob => {
                    generate_blob_decode(ident, col_name, &f.base_type, is_nullable, f.is_json)
                }
                FieldStorageKind::MySQLBlob => {
                    generate_mysql_blob_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::MySQLText => {
                    generate_mysql_text_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::MySQLColumn => {
                    generate_mysql_column_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::SQLiteColumn => {
                    generate_sqlite_column_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::SQLiteJson => {
                    generate_sqlite_json_decode(ident, col_name, &f.base_type, is_nullable)
                }
                FieldStorageKind::MySQLJson => {
                    generate_mysql_json_decode(ident, col_name, &f.base_type, is_nullable)
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
                    .map(|value| {
                        value.parse::<#base_type>().map_err(|e| {
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

/// Reads a field into `drizzle::core::query::RawJson` and applies `decode`,
/// an expression over the owned `raw` value. For nullable fields a JSON
/// `null` becomes `None` without running `decode`.
fn generate_raw_json_decode(
    ident: &Ident,
    col_name: &str,
    is_nullable: bool,
    decode: &TokenStream,
) -> TokenStream {
    if is_nullable {
        quote! {
            #col_name => {
                let raw = map.next_value::<drizzle::core::query::RawJson>()?;
                state.#ident = ::std::option::Option::Some(if raw.is_null() {
                    ::std::option::Option::None
                } else {
                    ::std::option::Option::Some({ #decode })
                });
                ::std::result::Result::Ok(true)
            }
        }
    } else {
        quote! {
            #col_name => {
                let raw = map.next_value::<drizzle::core::query::RawJson>()?;
                state.#ident = ::std::option::Option::Some({ #decode });
                ::std::result::Result::Ok(true)
            }
        }
    }
}

fn generate_sqlite_uuid_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        let value = drizzle::sqlite::traits::decode_projected_sqlite_value(&raw)
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': {e}", #col_name)
                )
            })?;
        let value = value.as_value();
        <#base_type as drizzle::sqlite::traits::FromSQLiteValue>::from_sqlite_ref(value.as_ref())
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': invalid UUID: {e}", #col_name)
                )
            })?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

/// Decodes a SQLite JSON column, which the projection embeds as a JSON string
/// holding the stored document.
fn generate_sqlite_json_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        drizzle::core::query::decode_json_text::<#base_type, __A::Error>(raw, #col_name)?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

/// Decodes a MySQL JSON column through the `Json<T>` column codec.
fn generate_mysql_json_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        drizzle::mysql::driver::decode_projected::<drizzle::core::Json<#base_type>>(&raw)
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': {e}", #col_name)
                )
            })?
            .into_inner()
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
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
            drizzle::core::query::decode_json_bytes::<#base_type, __A::Error>(&bytes, #col_name)?
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
        let projected = drizzle::sqlite::traits::decode_projected_sqlite_value(&raw)
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': {e}", #col_name)
                )
            })?;
        let drizzle::sqlite::values::OwnedSQLiteValue::Blob(bytes) = projected else {
            return ::std::result::Result::Err(
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': projected value is not a SQLite BLOB", #col_name)
                )
            );
        };
        #convert
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

fn generate_mysql_blob_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        drizzle::mysql::driver::decode_blob::<#base_type>(&raw)
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': {e}", #col_name)
                )
            })?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

fn generate_mysql_text_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        drizzle::mysql::driver::decode_text::<#base_type>(&raw)
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': {e}", #col_name)
                )
            })?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

fn generate_mysql_column_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        drizzle::mysql::driver::decode_projected::<#base_type>(&raw)
            .map_err(|e| {
                <__A::Error as drizzle::core::serde::de::Error>::custom(
                    ::std::format!("field '{}': {e}", #col_name)
                )
            })?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

fn generate_sqlite_column_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    let decode = quote! {
        <#base_type as drizzle::sqlite::traits::DrizzleSQLiteColumn>::decode_json(&raw)
        .map_err(|e| {
            <__A::Error as drizzle::core::serde::de::Error>::custom(
                ::std::format!("field '{}': {e}", #col_name)
            )
        })?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}

fn generate_enum_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    storage: EnumStorage,
    is_nullable: bool,
) -> TokenStream {
    if let EnumStorage::Postgres = storage {
        return generate_postgres_enum_decode(ident, col_name, base_type, is_nullable);
    }

    let decode_some = enum_json_decode(base_type, col_name, storage);
    let raw_type = match storage {
        EnumStorage::Integer => quote!(i64),
        EnumStorage::Text => quote!(::std::string::String),
        EnumStorage::Postgres => unreachable!(),
    };

    if is_nullable {
        quote! {
            #col_name => {
                let raw = map.next_value::<::std::option::Option<#raw_type>>()?;
                state.#ident = ::std::option::Option::Some(match raw {
                    ::std::option::Option::Some(raw) => ::std::option::Option::Some({ #decode_some }),
                    ::std::option::Option::None => ::std::option::Option::None,
                });
                ::std::result::Result::Ok(true)
            }
        }
    } else {
        quote! {
            #col_name => {
                let raw = map.next_value::<#raw_type>()?;
                state.#ident = ::std::option::Option::Some({ #decode_some });
                ::std::result::Result::Ok(true)
            }
        }
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
        EnumStorage::Postgres => unreachable!(),
    }
}

fn generate_postgres_enum_decode(
    ident: &Ident,
    col_name: &str,
    base_type: &syn::Type,
    is_nullable: bool,
) -> TokenStream {
    // Native enums are projected as strings, integer-backed enums as numbers.
    let decode = quote! {
        drizzle::core::query::decode_enum_value::<#base_type, __A::Error>(raw, #col_name)?
    };

    generate_raw_json_decode(ident, col_name, is_nullable, &decode)
}
