//! Common utilities shared across `SQLite` and `PostgreSQL` macro implementations.
//!
//! This module provides shared abstractions to reduce code duplication between
//! the dialect-specific macro implementations.

pub mod constraints;
mod context;
mod diagnostics;
pub mod enum_utils;
pub mod generators;
mod helpers;
#[cfg(feature = "query")]
pub mod query;
pub mod ref_gen;
mod table_pipeline;
pub mod type_mapping;
mod type_utils;
pub mod view_query;

pub use context::ModelType;
pub use diagnostics::references_required_message;
#[cfg(feature = "sqlite")]
pub use helpers::has_json_attribute;
pub use helpers::{extract_struct_fields, make_uppercase_path, parse_column_reference};
pub use table_pipeline::{
    count_primary_keys, required_fields_pattern, struct_fields, table_name_from_attrs,
};
pub use type_mapping::{generate_arithmetic_ops, generate_expr_impl, rust_type_to_nullability};
#[cfg(feature = "postgres")]
pub use type_mapping::{postgres_column_type_is_numeric, postgres_column_type_to_sql_type};
#[cfg(feature = "sqlite")]
pub use type_mapping::{sqlite_column_type_is_numeric, sqlite_column_type_to_sql_type};
#[cfg(feature = "sqlite")]
pub use type_utils::type_is_byte_slice;
pub use type_utils::{
    is_option_type, option_inner_type, type_is_array_string, type_is_array_u8, type_is_arrayvec_u8,
    type_is_bool, type_is_datetime_tz, type_is_float, type_is_int, type_is_json_value,
    type_is_naive_date, type_is_naive_datetime, type_is_naive_time, type_is_offset_datetime,
    type_is_primitive_date_time, type_is_string_like, type_is_time_date, type_is_time_time,
    type_is_uuid, type_is_vec_u8, unwrap_option,
};
#[cfg(feature = "postgres")]
pub use type_utils::{
    type_is_array_char, type_is_bit_vec, type_is_geo_linestring, type_is_geo_point,
    type_is_geo_rect, type_is_ip_addr, type_is_ip_cidr, type_is_mac_addr,
};

// Re-export dialect traits (always available)
#[allow(unused_imports)]
pub use generators::{Dialect, GeneratorPaths};

// Re-export dialect implementations (feature-gated)
#[cfg(feature = "sqlite")]
pub use generators::SqliteDialect;

#[cfg(feature = "postgres")]
pub use generators::PostgresDialect;
