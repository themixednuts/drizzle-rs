//! `PostgreSQL` types.
//!
//! - [`types`]: zero-sized SQL type markers used for compile-time checks.
//! - [`PostgreSQLType`]: column types as written in DDL, with their metadata.
//! - [`TypeCategory`]: how a Rust field type maps to a `PostgreSQL` column.
//! - [`PgTypeCategory`]: categories of SQL type names, used when parsing.
//! - [`ddl`]: schema objects (tables, columns, indexes, ...) for migrations.

pub mod ddl;
mod sql_type;
mod type_category;

/// Zero-sized SQL type markers for the `PostgreSQL` dialect.
///
/// Each marker stands for one SQL type at compile time; it carries no length,
/// precision or enum values (see [`PostgreSQLType`] for those). The traits in
/// [`crate::sql`] say which markers can be compared, assigned, or used in
/// arithmetic. The Rust types listed are the default value mappings.
pub mod types {
    /// `smallint` (`int2`): 16-bit integer. Rust `i16`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int2;

    /// `integer` (`int4`): 32-bit integer. Rust `i32`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int4;

    /// `bigint` (`int8`): 64-bit integer. Rust `i64`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int8;

    /// `real` (`float4`): 32-bit float. Rust `f32`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Float4;

    /// `double precision` (`float8`): 64-bit float. Rust `f64`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Float8;

    /// `varchar(n)`: text with a length limit.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Varchar;

    /// `text`: unlimited text. Rust `&str`, `String`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Text;

    /// `char(n)`: fixed-length, blank-padded text.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Char;

    /// `bytea`: raw bytes. Rust `&[u8]`, `Vec<u8>`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Bytea;

    /// `boolean`. Rust `bool`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Boolean;

    /// `timestamp with time zone`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timestamptz;

    /// `timestamp` (without time zone).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timestamp;

    /// `date`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Date;

    /// `time` (without time zone).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Time;

    /// `time with time zone`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timetz;

    /// `numeric(p, s)`: exact decimal.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Numeric;

    /// `uuid`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Uuid;

    /// `json`: JSON stored as text.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Json;

    /// `jsonb`: binary JSON; supports containment and key operators.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Jsonb;

    /// Untyped SQL, such as a raw `SQL` fragment. Compatible with every `PostgreSQL` marker.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Any;

    /// `interval`: a time span.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Interval;

    /// `inet`: an IPv4 or IPv6 host address, with optional netmask.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Inet;

    /// `cidr`: an IPv4 or IPv6 network.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Cidr;

    /// `macaddr`: a 6-byte MAC address.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct MacAddr;

    /// `macaddr8`: an 8-byte MAC address.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct MacAddr8;

    /// `point`: a geometric point.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Point;

    /// `path`: an open or closed geometric path.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct LineString;

    /// `box`: a rectangle.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Rect;

    /// `bit(n)` / `bit varying(n)`: a bit string.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct BitString;

    /// `line`: an infinite geometric line.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Line;

    /// `lseg`: a line segment.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct LineSegment;

    /// `polygon`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Polygon;

    /// `circle`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Circle;

    /// A user-defined enum type (`CREATE TYPE ... AS ENUM`).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Enum;
}

pub use sql_type::PostgreSQLType;
pub use type_category::{PgTypeCategory, TypeCategory};
