//! Attribute markers for PostgresTable derive macro.
//!
//! These const markers are used within `#[column(...)]` and `#[PostgresTable(...)]`
//! attributes. Import them from the prelude to get IDE hover documentation.
//!
//! # Example
//! ```ignore
//! use drizzle::postgres::prelude::*;
//!
//! #[PostgresTable(NAME = "users")]
//! struct User {
//!     #[column(PRIMARY, SERIAL)]
//!     id: i32,
//!     #[column(UNIQUE)]
//!     email: String,
//!     #[column(JSON)]
//!     metadata: Option<Metadata>,
//! }
//! ```

/// Marker struct for column constraint attributes.
#[derive(Debug, Clone, Copy)]
pub struct ColumnMarker;

//------------------------------------------------------------------------------
// Primary Key Constraints
//------------------------------------------------------------------------------

/// Marks this column as the PRIMARY KEY.
///
/// ## Example
/// ```ignore
/// #[column(PRIMARY)]
/// id: i32,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-PRIMARY-KEYS>
pub const PRIMARY: ColumnMarker = ColumnMarker;

/// Alias for [`PRIMARY`].
pub const PRIMARY_KEY: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Auto-increment Types
//------------------------------------------------------------------------------

/// Creates a SERIAL column (auto-incrementing 32-bit integer).
///
/// ## Example
/// ```ignore
/// #[column(PRIMARY, SERIAL)]
/// id: i32,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
pub const SERIAL: ColumnMarker = ColumnMarker;

/// Creates a BIGSERIAL column (auto-incrementing 64-bit integer).
///
/// ## Example
/// ```ignore
/// #[column(PRIMARY, BIGSERIAL)]
/// id: i64,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
pub const BIGSERIAL: ColumnMarker = ColumnMarker;

/// Creates a SMALLSERIAL column (auto-incrementing 16-bit integer).
///
/// ## Example
/// ```ignore
/// #[column(PRIMARY, SMALLSERIAL)]
/// id: i16,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
pub const SMALLSERIAL: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Uniqueness Constraints
//------------------------------------------------------------------------------

/// Adds a UNIQUE constraint to this column.
///
/// ## Example
/// ```ignore
/// #[column(UNIQUE)]
/// email: String,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS>
pub const UNIQUE: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Identity Columns
//------------------------------------------------------------------------------

/// Creates a GENERATED ALWAYS AS IDENTITY column.
///
/// ## Example
/// ```ignore
/// #[column(GENERATED_IDENTITY)]
/// id: i64,
/// ```
///
/// ## Technical Details
/// PostgreSQL's identity columns are SQL-standard compliant, unlike SERIAL.
/// Use this for new schemas when possible.
///
/// See: <https://www.postgresql.org/docs/current/ddl-identity-columns.html>
pub const GENERATED_IDENTITY: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Serialization Modes
//------------------------------------------------------------------------------

/// Enables JSON serialization with JSON type storage.
///
/// ## Example
/// ```ignore
/// #[column(JSON)]
/// metadata: UserMetadata,
/// ```
///
/// Requires the `serde` feature. The field type must implement `Serialize` and `Deserialize`.
///
/// See: <https://www.postgresql.org/docs/current/datatype-json.html>
pub const JSON: ColumnMarker = ColumnMarker;

/// Enables JSON serialization with JSONB storage.
///
/// ## Example
/// ```ignore
/// #[column(JSONB)]
/// config: AppConfig,
/// ```
///
/// JSONB is the recommended JSON storage format for most use cases.
/// It supports indexing and efficient querying.
///
/// Requires the `serde` feature. The field type must implement `Serialize` and `Deserialize`.
///
/// See: <https://www.postgresql.org/docs/current/datatype-json.html>
pub const JSONB: ColumnMarker = ColumnMarker;

/// Marks this column as storing an enum type.
///
/// ## Example
/// ```ignore
/// #[column(ENUM)]
/// role: Role,
/// ```
///
/// For PostgreSQL native ENUM types or text-based enum storage.
///
/// See: <https://www.postgresql.org/docs/current/datatype-enum.html>
pub const ENUM: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Default Value Parameters
//------------------------------------------------------------------------------

/// Specifies a function to generate default values at runtime.
///
/// The function is called for each insert when no value is provided.
///
/// ## Example
/// ```ignore
/// #[column(DEFAULT_FN = Uuid::new_v4)]
/// id: Uuid,
/// ```
///
/// ## Difference from DEFAULT
/// - `DEFAULT_FN`: Calls the function at runtime for each insert (e.g., UUID generation)
/// - `DEFAULT`: Uses a fixed compile-time value
pub const DEFAULT_FN: ColumnMarker = ColumnMarker;

/// Specifies a fixed default value for new rows.
///
/// ## Example
/// ```ignore
/// #[column(DEFAULT = 0)]
/// count: i32,
///
/// #[column(DEFAULT = "guest")]
/// role: String,
/// ```
///
/// For runtime-generated values (UUIDs, timestamps), use [`DEFAULT_FN`] instead.
///
/// See: <https://www.postgresql.org/docs/current/ddl-default.html>
pub const DEFAULT: ColumnMarker = ColumnMarker;

/// Establishes a foreign key reference to another table's column.
///
/// ## Example
/// ```ignore
/// #[column(REFERENCES = User::id)]
/// user_id: i32,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-FK>
pub const REFERENCES: ColumnMarker = ColumnMarker;

/// Adds a CHECK constraint to this column.
///
/// ## Example
/// ```ignore
/// #[column(CHECK = "age >= 0")]
/// age: i32,
/// ```
///
/// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-CHECK-CONSTRAINTS>
pub const CHECK: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Name Marker (shared by column and table attributes)
//------------------------------------------------------------------------------

/// Marker struct for the NAME attribute.
#[derive(Debug, Clone, Copy)]
pub struct NameMarker;

/// Specifies a custom name in the database.
///
/// ## Column Example
/// ```ignore
/// #[column(NAME = "created_at")]
/// created: DateTime<Utc>,
/// ```
///
/// ## Table Example
/// ```ignore
/// #[PostgresTable(NAME = "user_accounts")]
/// struct User { ... }
/// ```
pub const NAME: NameMarker = NameMarker;

//------------------------------------------------------------------------------
// Table Attribute Markers
//------------------------------------------------------------------------------

/// Marker struct for table-level attributes.
#[derive(Debug, Clone, Copy)]
pub struct TableMarker;

/// Creates an UNLOGGED table.
///
/// ## Example
/// ```ignore
/// #[PostgresTable(UNLOGGED)]
/// struct SessionCache {
///     #[column(PRIMARY)]
///     key: String,
///     data: String,
/// }
/// ```
///
/// Unlogged tables are faster but data is not crash-safe.
///
/// See: <https://www.postgresql.org/docs/current/sql-createtable.html#SQL-CREATETABLE-UNLOGGED>
pub const UNLOGGED: TableMarker = TableMarker;

/// Creates a TEMPORARY table.
///
/// ## Example
/// ```ignore
/// #[PostgresTable(TEMPORARY)]
/// struct TempData {
///     id: i32,
///     value: String,
/// }
/// ```
///
/// Temporary tables exist only for the current session.
///
/// See: <https://www.postgresql.org/docs/current/sql-createtable.html#SQL-CREATETABLE-TEMPORARY>
pub const TEMPORARY: TableMarker = TableMarker;

/// Specifies inheritance from a parent table.
///
/// ## Example
/// ```ignore
/// #[PostgresTable(INHERITS = "base_table")]
/// struct ChildTable {
///     extra_field: String,
/// }
/// ```
///
/// See: <https://www.postgresql.org/docs/current/ddl-inherit.html>
pub const INHERITS: TableMarker = TableMarker;

/// Specifies a tablespace for the table.
///
/// ## Example
/// ```ignore
/// #[PostgresTable(TABLESPACE = "fast_storage")]
/// struct HighPerfTable {
///     #[column(PRIMARY)]
///     id: i32,
/// }
/// ```
///
/// See: <https://www.postgresql.org/docs/current/sql-createtable.html#SQL-CREATETABLE-TABLESPACE>
pub const TABLESPACE: TableMarker = TableMarker;
