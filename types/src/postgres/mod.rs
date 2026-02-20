//! PostgreSQL type definitions
//!
//! This module provides type definitions for PostgreSQL including:
//!
//! - [`PostgreSQLType`] - PostgreSQL column types
//! - [`TypeCategory`] - Rust type classification for PostgreSQL mapping
//! - [`PgTypeCategory`] - SQL type categories for parsing

pub mod ddl;
mod sql_type;
mod type_category;

pub mod types {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int2;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int4;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int8;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Float4;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Float8;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Varchar;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Text;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Char;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Bytea;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Boolean;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timestamptz;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timestamp;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Date;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Time;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timetz;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Numeric;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Uuid;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Json;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Jsonb;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Any;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Interval;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Inet;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Cidr;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct MacAddr;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct MacAddr8;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Point;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct LineString;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Rect;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct BitString;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Line;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct LineSegment;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Polygon;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Circle;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Enum;
}

pub use sql_type::PostgreSQLType;
pub use type_category::{PgTypeCategory, TypeCategory};
