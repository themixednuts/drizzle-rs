//! Attribute markers for SQLiteTable derive macro.
//!
//! These const markers are used within `#[column(...)]` and `#[SQLiteTable(...)]`
//! attributes. Import them from the prelude to get IDE hover documentation.
//!
//! # Example
//! ```ignore
//! use drizzle::sqlite::prelude::*;
//!
//! #[SQLiteTable(name = "users", strict)]
//! struct User {
//!     #[column(primary, autoincrement)]
//!     id: i32,
//!     #[column(unique)]
//!     email: String,
//!     #[column(json)]
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
/// #[column(primary)]
/// id: i32,
/// ```
///
/// See: <https://sqlite.org/lang_createtable.html#primkeyconst>
pub const PRIMARY: ColumnMarker = ColumnMarker;

/// Alias for [`PRIMARY`].
pub const PRIMARY_KEY: ColumnMarker = ColumnMarker;

/// Enables AUTOINCREMENT for INTEGER PRIMARY KEY columns.
///
/// ## Example
/// ```ignore
/// #[column(primary, autoincrement)]
/// id: i32,
/// ```
///
/// See: <https://sqlite.org/autoinc.html>
pub const AUTOINCREMENT: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Uniqueness Constraints
//------------------------------------------------------------------------------

/// Adds a UNIQUE constraint to this column.
///
/// ## Example
/// ```ignore
/// #[column(unique)]
/// email: String,
/// ```
///
/// See: <https://sqlite.org/lang_createtable.html#unique_constraints>
pub const UNIQUE: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Serialization Modes
//------------------------------------------------------------------------------

/// Enables JSON serialization with TEXT storage.
///
/// ## Example
/// ```ignore
/// #[column(json)]
/// metadata: UserMetadata,
/// ```
///
/// Requires the `serde` feature. The field type must implement `Serialize` and `Deserialize`.
pub const JSON: ColumnMarker = ColumnMarker;

/// Enables JSON serialization with BLOB storage.
///
/// ## Example
/// ```ignore
/// #[column(jsonb)]
/// config: AppConfig,
/// ```
///
/// Requires the `serde` feature. The field type must implement `Serialize` and `Deserialize`.
pub const JSONB: ColumnMarker = ColumnMarker;

/// Marks this column as storing an enum type.
///
/// ## Example
/// ```ignore
/// #[column(enum)]
/// role: Role,
///
/// #[column(integer, enum)]
/// status: Status,
/// ```
///
/// The enum must derive `SQLiteEnum`.
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
/// #[column(default_fn = Uuid::new_v4)]
/// id: Uuid,
/// ```
///
/// ## Difference from DEFAULT
/// - `default_fn`: Calls the function at runtime for each insert (e.g., UUID generation)
/// - `default`: Uses a fixed compile-time value
pub const DEFAULT_FN: ColumnMarker = ColumnMarker;

/// Specifies a fixed default value for new rows.
///
/// ## Example
/// ```ignore
/// #[column(default = 0)]
/// count: i32,
///
/// #[column(default = "guest")]
/// role: String,
/// ```
///
/// For runtime-generated values (UUIDs, timestamps), use [`DEFAULT_FN`] instead.
pub const DEFAULT: ColumnMarker = ColumnMarker;

/// Establishes a foreign key reference to another table's column.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id)]
/// user_id: i32,
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html>
pub const REFERENCES: ColumnMarker = ColumnMarker;

/// Specifies the ON DELETE action for foreign key references.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_delete = CASCADE)]
/// user_id: i32,
/// ```
///
/// ## Supported Actions
/// - `CASCADE`: Delete rows that reference the deleted row
/// - `SET_NULL`: Set the column to NULL when referenced row is deleted
/// - `SET_DEFAULT`: Set the column to its default value
/// - `RESTRICT`: Prevent deletion if referenced
/// - `NO_ACTION`: Similar to RESTRICT (default)
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const ON_DELETE: ColumnMarker = ColumnMarker;

/// Specifies the ON UPDATE action for foreign key references.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_update = CASCADE)]
/// user_id: i32,
/// ```
///
/// ## Supported Actions
/// - `CASCADE`: Update referencing rows when referenced row is updated
/// - `SET_NULL`: Set the column to NULL when referenced row is updated
/// - `SET_DEFAULT`: Set the column to its default value
/// - `RESTRICT`: Prevent update if referenced
/// - `NO_ACTION`: Similar to RESTRICT (default)
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const ON_UPDATE: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Referential Action Values
//------------------------------------------------------------------------------

/// Marker struct for referential action values used with on_delete/on_update.
#[derive(Debug, Clone, Copy)]
pub struct ReferentialAction;

/// CASCADE action: Propagate the delete/update to referencing rows.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_delete = CASCADE)]
/// user_id: i32,
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const CASCADE: ReferentialAction = ReferentialAction;

/// SET NULL action: Set referencing columns to NULL.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_delete = SET_NULL)]
/// user_id: Option<i32>,
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const SET_NULL: ReferentialAction = ReferentialAction;

/// SET DEFAULT action: Set referencing columns to their default values.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_delete = SET_DEFAULT, default = 0)]
/// user_id: i32,
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const SET_DEFAULT: ReferentialAction = ReferentialAction;

/// RESTRICT action: Prevent delete/update if referenced.
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_delete = RESTRICT)]
/// user_id: i32,
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const RESTRICT: ReferentialAction = ReferentialAction;

/// NO ACTION action: Similar to RESTRICT (default behavior).
///
/// ## Example
/// ```ignore
/// #[column(references = User::id, on_delete = NO_ACTION)]
/// user_id: i32,
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const NO_ACTION: ReferentialAction = ReferentialAction;

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
/// #[column(name = "created_at")]
/// created: DateTime<Utc>,
/// ```
///
/// ## Table Example
/// ```ignore
/// #[SQLiteTable(name = "user_accounts")]
/// struct User { ... }
/// ```
pub const NAME: NameMarker = NameMarker;

//------------------------------------------------------------------------------
// Table Attribute Markers
//------------------------------------------------------------------------------

/// Marker struct for table-level attributes.
#[derive(Debug, Clone, Copy)]
pub struct TableMarker;

/// Enables STRICT mode for the table.
///
/// ## Example
/// ```ignore
/// #[SQLiteTable(strict)]
/// struct Users {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
/// ```
///
/// # SQLite Behavior
/// - Enforces that values match declared column types exactly
/// - `INTEGER` columns only accept integers
/// - `TEXT` columns only accept text
/// - `REAL` columns only accept floating-point numbers
/// - `BLOB` columns only accept blobs
/// - `ANY` type allows any value (only in STRICT tables)
///
/// See: <https://sqlite.org/stricttables.html>
pub const STRICT: TableMarker = TableMarker;

/// Enables WITHOUT ROWID optimization for the table.
///
/// ## Example
/// ```ignore
/// #[SQLiteTable(without_rowid)]
/// struct KeyValue {
///     #[column(primary)]
///     key: String,
///     value: String,
/// }
/// ```
///
/// Requires an explicit PRIMARY KEY.
///
/// See: <https://sqlite.org/withoutrowid.html>
pub const WITHOUT_ROWID: TableMarker = TableMarker;
