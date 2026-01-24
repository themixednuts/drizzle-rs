//! Common utilities shared across SQLite and PostgreSQL macro implementations.
//!
//! This module provides shared abstractions to reduce code duplication between
//! the dialect-specific macro implementations.

mod context;
mod diagnostics;
pub(crate) mod generators;
mod helpers;
mod table_pipeline;
pub(crate) mod type_mapping;
mod type_utils;

pub(crate) use context::ModelType;
pub(crate) use diagnostics::references_required_message;
pub(crate) use helpers::{
    extract_struct_fields, generate_try_from_impl, make_uppercase_path, parse_column_reference,
};
pub(crate) use table_pipeline::{
    count_primary_keys, required_fields_pattern, struct_fields, table_name_from_attrs,
};
pub(crate) use type_mapping::{
    generate_arithmetic_ops, generate_expr_impl, is_numeric_sql_type, rust_type_to_nullability,
    rust_type_to_sql_type,
};
pub(crate) use type_utils::{
    is_option_type, option_inner_type, type_is_array_char, type_is_array_string, type_is_array_u8,
    type_is_arrayvec_u8, type_is_bit_vec, type_is_bool, type_is_byte_slice, type_is_char_array,
    type_is_chrono_date, type_is_chrono_datetime, type_is_chrono_time, type_is_datetime_tz,
    type_is_float, type_is_geo_linestring, type_is_geo_point, type_is_geo_rect, type_is_int,
    type_is_ip_addr, type_is_ip_cidr, type_is_json_value, type_is_mac_addr, type_is_naive_date,
    type_is_naive_datetime, type_is_naive_time, type_is_offset_datetime,
    type_is_primitive_date_time, type_is_string_like, type_is_time_date, type_is_time_time,
    type_is_uuid, type_is_vec_u8, unwrap_option,
};

// Re-export dialect traits (always available)
#[allow(unused_imports)]
pub(crate) use generators::{Dialect, GeneratorPaths};

// Re-export dialect implementations (feature-gated)
#[cfg(feature = "sqlite")]
pub(crate) use generators::SqliteDialect;

#[cfg(feature = "postgres")]
pub(crate) use generators::PostgresDialect;
