//! Names accepted inside `#[SQLiteTable(...)]`, `#[column(...)]`,
//! `#[SQLiteView(...)]` and `#[SQLiteIndex(...)]`.
//!
//! The macros read these attributes by name, case-insensitively
//! (`primary` and `PRIMARY` are the same). The constants here exist so your
//! editor can show their documentation on hover; the macros point each
//! attribute at its constant, so import them through the prelude.
//!
//! # Examples
//!
//! ```rust
//! # mod drizzle {
//! #     pub mod core { pub use drizzle_core::*; }
//! #     pub mod error { pub use drizzle_core::error::*; }
//! #     pub mod types { pub use drizzle_types::*; }
//! #     pub mod migrations { pub use drizzle_migrations::*; }
//! #     pub use drizzle_types::Dialect;
//! #     pub use drizzle_types as ddl;
//! #     pub mod sqlite {
//! #         pub use drizzle_sqlite::{*, attrs::*};
//! #         #[cfg(feature = "rusqlite")]
//! #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
//! #         #[cfg(feature = "libsql")]
//! #         pub mod libsql { pub use ::libsql::{Row, Value}; }
//! #         #[cfg(feature = "turso")]
//! #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
//! #         pub mod prelude {
//! #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
//! #             pub use drizzle_sqlite::{*, attrs::*};
//! #             pub use drizzle_core::*;
//! #         }
//! #     }
//! # }
//! use drizzle::sqlite::prelude::*;
//!
//! #[SQLiteTable(
//!     name = "users",
//!     strict,
//!     unique(columns(email, tenant_id)),
//!     check(name = "users_score_check", expr = "score >= 0")
//! )]
//! struct User {
//!     #[column(primary, autoincrement)]
//!     id: i32,
//!     #[column(unique, collate = NOCASE)]
//!     email: String,
//!     tenant_id: i32,
//!     #[column(default = 0)]
//!     score: i32,
//! }
//!
//! #[SQLiteTable(name = "posts")]
//! struct Post {
//!     #[column(primary)]
//!     id: i32,
//!     #[column(references = User::id, on_delete = CASCADE)]
//!     author_id: i32,
//!     title: String,
//! }
//! ```
//!
//! The per-attribute examples below are fragments of such a definition.

/// The type of the column constraint and option constants.
#[derive(Debug, Clone, Copy)]
pub struct ColumnMarker;

//------------------------------------------------------------------------------
// Primary Key Constraints
//------------------------------------------------------------------------------

/// Marks this column as the PRIMARY KEY.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(primary)]
/// id: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/lang_createtable.html#primkeyconst>
pub const PRIMARY: ColumnMarker = ColumnMarker;

/// Alias for [`PRIMARY`].
pub const PRIMARY_KEY: ColumnMarker = ColumnMarker;

/// Adds `AUTOINCREMENT` to an `INTEGER PRIMARY KEY` column, so rowids of
/// deleted rows are never reused.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(primary, autoincrement)]
/// id: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/autoinc.html>
pub const AUTOINCREMENT: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Index Attributes
//------------------------------------------------------------------------------

/// The type of the index option constants.
#[derive(Debug, Clone, Copy)]
pub struct IndexMarker;

/// Makes an index partial: only rows matching the SQL predicate are indexed.
///
/// The predicate is raw SQL. Write database column names; renaming a Rust
/// field does not rewrite it.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteIndex(where = "deleted_at IS NULL")]
/// struct ActiveUsersEmailIdx(Users::email);
/// # "####;
/// ```
///
/// See: <https://sqlite.org/partialindex.html>
pub const WHERE: IndexMarker = IndexMarker;

//------------------------------------------------------------------------------
// Uniqueness Constraints
//------------------------------------------------------------------------------

/// Adds a UNIQUE constraint to a column, table, or index.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(unique)]
/// email: String,
///
/// #[SQLiteTable(unique(columns(email, tenant_id)))]
/// struct Users {
///     email: String,
///     tenant_id: i32,
/// }
///
/// #[SQLiteIndex(unique)]
/// struct UsersEmailIdx(Users::email);
/// # "####;
/// ```
///
/// See: <https://sqlite.org/lang_createtable.html#unique_constraints>
pub const UNIQUE: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Serialization Modes
//------------------------------------------------------------------------------

/// Stores the field as JSON text, serialized with serde.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(json)]
/// metadata: UserMetadata,
/// # "####;
/// ```
///
/// Requires the `serde` feature. The field type must implement `Serialize`
/// and `Deserialize`. Values are bound through `json(?)`.
pub const JSON: ColumnMarker = ColumnMarker;

/// Stores a `#[derive(SQLiteEnum)]` enum.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(enum)]
/// role: Role,
///
/// #[column(integer, enum)]
/// status: Status,
/// # "####;
/// ```
///
/// The enum must derive `SQLiteEnum`, and that derive decides the storage:
/// INTEGER when a variant has an explicit discriminant or the enum has an
/// integer `#[repr]`, TEXT (variant names) otherwise. An explicit `integer` or
/// `text` marker must agree with it, or the table fails to compile.
pub const ENUM: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Default Value Parameters
//------------------------------------------------------------------------------

/// Generates a value in Rust for each insert that leaves the column unset.
///
/// The function takes no arguments and returns the field type.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(default_fn = Uuid::new_v4)]
/// id: Uuid,
/// # "####;
/// ```
///
/// Unlike [`DEFAULT`], this does not add a database `DEFAULT` clause.
pub const DEFAULT_FN: ColumnMarker = ColumnMarker;

/// Adds a `DEFAULT` clause to the column.
///
/// Takes a literal, `CURRENT_TIME`, `CURRENT_DATE`, `CURRENT_TIMESTAMP`, or
/// an SQL function call. Expressions other than literals and the `CURRENT_*`
/// keywords are wrapped in parentheses, as SQLite requires.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(default = 0)]
/// count: i32,
///
/// #[column(default = "guest")]
/// role: String,
///
/// #[column(default = CURRENT_TIMESTAMP)]
/// created_at: String,
///
/// #[column(default = strftime("%s", "now"))]
/// created_at_unix: i64,
/// # "####;
/// ```
///
/// For application-generated values such as UUIDs, use [`DEFAULT_FN`] instead.
///
/// See: <https://sqlite.org/lang_createtable.html#the_default_clause>
pub const DEFAULT: ColumnMarker = ColumnMarker;

/// Makes the column a generated column: `stored` (computed on write) or
/// `virtual` (computed on read), from a raw SQL expression.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(generated(stored, "length(name)"))]
/// stored_name_len: i32,
///
/// #[column(generated(virtual, "length(name)"))]
/// virtual_name_len: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/gencol.html>
pub const GENERATED: ColumnMarker = ColumnMarker;

/// Adds a CHECK constraint to a column (`check = "..."`) or a table
/// (`check(name = "...", expr = "...")`). The expression is raw SQL.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(check = "score >= 0")]
/// score: i32,
///
/// #[SQLiteTable(check(name = "score_range", expr = "score >= 0 AND score <= 100"))]
/// struct Scores {
///     score: i32,
/// }
/// # "####;
/// ```
///
/// See: <https://sqlite.org/lang_createtable.html#check_constraints>
pub const CHECK: ColumnMarker = ColumnMarker;

/// Adds a foreign key that references a column of another table.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id)]
/// user_id: i32,
/// # "####;
/// ```
///
/// With the `query` feature this also generates relation accessors: a
/// forward one on this table, named after the column without its `_id` suffix
/// (`user_id` gives `.user()`), and a reverse one on the referenced table
/// (see [`RELATION`]).
///
/// See: <https://sqlite.org/foreignkeys.html>
pub const REFERENCES: ColumnMarker = ColumnMarker;

/// Sets the reverse relation accessor name on the referenced table.
///
/// By default, reverse relations are named from the source table
/// (`posts` for a `Post` table). When multiple foreign keys target the
/// same table — or the FK is self-referential — the name is disambiguated
/// as `{forward}_{plural}` (e.g. `author_posts`). Use `relation` to pick
/// an explicit reverse name instead.
///
/// The forward relation (on this table) is unchanged; only the reverse
/// accessor on the referenced table is renamed.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// // Users get `.authored()` instead of `.author_posts()`
/// #[column(references = User::id, relation = "authored")]
/// author_id: i32,
///
/// // Still auto-disambiguated: Users get `.editor_posts()`
/// #[column(references = User::id)]
/// editor_id: Option<i32>,
/// # "####;
/// ```
///
/// Requires a `references` attribute on the same column.
pub const RELATION: ColumnMarker = ColumnMarker;

/// Sets the `ON DELETE` action of a foreign key.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_delete = CASCADE)]
/// user_id: i32,
/// # "####;
/// ```
///
/// ## Supported Actions
/// - `CASCADE`: Delete rows that reference the deleted row
/// - `SET_NULL`: Set the column to NULL when referenced row is deleted
/// - `SET_DEFAULT`: Set the column to its default value
/// - `RESTRICT`: Prevent deletion if referenced
/// - `NO_ACTION`: Like `RESTRICT`, but checked at the end of the statement
///   (the default)
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const ON_DELETE: ColumnMarker = ColumnMarker;

/// Sets the `ON UPDATE` action of a foreign key.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_update = CASCADE)]
/// user_id: i32,
/// # "####;
/// ```
///
/// ## Supported Actions
/// - `CASCADE`: Update referencing rows when referenced row is updated
/// - `SET_NULL`: Set the column to NULL when referenced row is updated
/// - `SET_DEFAULT`: Set the column to its default value
/// - `RESTRICT`: Prevent update if referenced
/// - `NO_ACTION`: Like `RESTRICT`, but checked at the end of the statement
///   (the default)
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const ON_UPDATE: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Referential Action Values
//------------------------------------------------------------------------------

/// The type of the referential action constants ([`CASCADE`], [`SET_NULL`], ...).
pub type ReferentialAction = ColumnMarker;

/// `CASCADE`: delete or update the referencing rows too.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_delete = CASCADE)]
/// user_id: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const CASCADE: ColumnMarker = ColumnMarker;

/// `SET NULL`: set the referencing columns to NULL.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_delete = SET_NULL)]
/// user_id: Option<i32>,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const SET_NULL: ColumnMarker = ColumnMarker;

/// `SET DEFAULT`: set the referencing columns to their defaults.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_delete = SET_DEFAULT, default = 0)]
/// user_id: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const SET_DEFAULT: ColumnMarker = ColumnMarker;

/// `RESTRICT`: reject the delete or update while rows reference it.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_delete = RESTRICT)]
/// user_id: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const RESTRICT: ColumnMarker = ColumnMarker;

/// `NO ACTION`: like `RESTRICT`, but checked at the end of the statement.
/// The default.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(references = User::id, on_delete = NO_ACTION)]
/// user_id: i32,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_actions>
pub const NO_ACTION: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Collation Markers
//------------------------------------------------------------------------------

/// Sets the collation of a text column.
///
/// Takes `BINARY`, `NOCASE`, `RTRIM`, or the name of a collation the
/// application registers, as a string.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(COLLATE = NOCASE)]
/// name: String,
///
/// // String form for custom registered collations:
/// #[column(COLLATE = "my_collation")]
/// label: String,
/// # "####;
/// ```
///
/// See: <https://sqlite.org/datatype3.html#collation>
pub const COLLATE: ColumnMarker = ColumnMarker;

/// BINARY collation: bytewise comparison of operands. The default for `BLOB`
/// columns and any column without an explicit collation.
pub const BINARY: ColumnMarker = ColumnMarker;

/// NOCASE collation: compares ASCII letters case-insensitively.
pub const NOCASE: ColumnMarker = ColumnMarker;

/// RTRIM collation: like `BINARY` but trailing spaces are ignored when
/// comparing.
pub const RTRIM: ColumnMarker = ColumnMarker;

//------------------------------------------------------------------------------
// Name Marker (shared by column and table attributes)
//------------------------------------------------------------------------------

/// The type of the [`NAME`] constant.
#[derive(Debug, Clone, Copy)]
pub struct NameMarker;

/// Sets the name used in the database.
///
/// By default, table, view and column names are the `snake_case` form of the
/// Rust struct or field name. `name` overrides that.
///
/// ## Column Example
/// ```rust
/// # let _ = r####"
/// // Column `created_at` by default; stored as `creation_timestamp` here.
/// #[column(name = "creation_timestamp")]
/// created_at: DateTime<Utc>,
/// # "####;
/// ```
///
/// ## Table Example
/// ```rust
/// # let _ = r####"
/// // Struct `UserAccount` becomes table `user_account` by default
/// struct UserAccount { ... }
///
/// // Override with custom name:
/// #[SQLiteTable(name = "user_accounts")]
/// struct UserAccount { ... }
/// # "####;
/// ```
///
/// ## View Example
/// ```rust
/// # let _ = r####"
/// #[SQLiteView(NAME = "active_users")]
/// struct ActiveUsers { ... }
/// # "####;
/// ```
pub const NAME: NameMarker = NameMarker;

//------------------------------------------------------------------------------
// View Attribute Markers
//------------------------------------------------------------------------------

/// The type of the view option constants.
#[derive(Debug, Clone, Copy)]
pub struct ViewMarker;

/// The view's query: an SQL string, or a block that returns a query
/// builder.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteView(DEFINITION = "SELECT id, email FROM users")]
/// struct UserEmails { id: i32, email: String }
/// # "####;
/// ```
///
/// ```rust
/// # let _ = r####"
/// #[SQLiteView(
///     DEFINITION = {
///         let builder = drizzle::sqlite::QueryBuilder::new::<Schema>();
///         let Schema { user } = Schema::new();
///         builder.select((user.id, user.email)).from(user)
///     }
/// )]
/// struct UserEmails { id: i32, email: String }
/// # "####;
/// ```
pub const DEFINITION: ViewMarker = ViewMarker;

/// Marks the view as already existing, so migrations do not create it.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteView(EXISTING)]
/// struct ExistingView { ... }
/// # "####;
/// ```
pub const EXISTING: ViewMarker = ViewMarker;

//------------------------------------------------------------------------------
// Table Attribute Markers
//------------------------------------------------------------------------------

/// The type of the table option constants.
#[derive(Debug, Clone, Copy)]
pub struct TableMarker;

/// Adds a table-level foreign key, for keys over several columns.
///
/// `on_delete` and `on_update` take the action as a string here.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteTable(foreign_key(
///     columns(tenant_id, user_id),
///     references(Users, tenant_id, id),
///     on_delete = "CASCADE"
/// ))]
/// struct Posts {
///     tenant_id: i32,
///     user_id: i32,
/// }
/// # "####;
/// ```
///
/// See: <https://sqlite.org/foreignkeys.html#fk_composite>
pub const FOREIGN_KEY: TableMarker = TableMarker;

/// Makes the table `STRICT`, so SQLite rejects values that do not match
/// the declared column types.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteTable(strict)]
/// struct Users {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
/// # "####;
/// ```
///
/// A STRICT table still converts values losslessly where it can (the text
/// `'1'` into an `INTEGER` column), and only `ANY` columns accept any value.
///
/// See: <https://sqlite.org/stricttables.html>
pub const STRICT: TableMarker = TableMarker;

/// Makes the table `WITHOUT ROWID`, stored as a clustered index on its
/// primary key.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteTable(without_rowid)]
/// struct KeyValue {
///     #[column(primary)]
///     key: String,
///     value: String,
/// }
/// # "####;
/// ```
///
/// Requires an explicit PRIMARY KEY.
///
/// See: <https://sqlite.org/withoutrowid.html>
pub const WITHOUT_ROWID: TableMarker = TableMarker;

//------------------------------------------------------------------------------
// Column Type Markers
//------------------------------------------------------------------------------

/// The type of the column type constants.
#[derive(Debug, Clone, Copy)]
pub struct TypeMarker;

/// Sets the column type to `INTEGER`.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(integer, primary)]
/// id: i32,
/// # "####;
/// ```
///
/// INTEGER columns store signed integers up to 8 bytes (64-bit).
/// `SQLite` uses a variable-length encoding, so small values use less space.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
pub const INTEGER: TypeMarker = TypeMarker;

/// Sets the column type to `TEXT`.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(text)]
/// name: String,
/// # "####;
/// ```
///
/// TEXT columns store strings in the database encoding (UTF-8 by default).
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
pub const TEXT: TypeMarker = TypeMarker;

/// Sets the column type to `BLOB`.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(blob)]
/// data: Vec<u8>,
/// # "####;
/// ```
///
/// BLOB columns store bytes exactly as given.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
pub const BLOB: TypeMarker = TypeMarker;

/// Sets the column type to `REAL`.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(real)]
/// price: f64,
/// # "####;
/// ```
///
/// REAL columns store 8-byte IEEE 754 floating-point numbers.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
pub const REAL: TypeMarker = TypeMarker;

/// Sets the column type to `NUMERIC`.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(numeric)]
/// amount: f64,
/// # "####;
/// ```
///
/// A NUMERIC column converts text that looks like a number into INTEGER or
/// REAL, and stores other values as given.
///
/// See: <https://sqlite.org/datatype3.html#type_affinity>
pub const NUMERIC: TypeMarker = TypeMarker;

/// Sets the column type to `ANY` (STRICT tables only).
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[SQLiteTable(strict)]
/// struct Data {
///     #[column(any)]
///     value: serde_json::Value,
/// }
/// # "####;
/// ```
///
/// An ANY column stores any value without conversion.
///
/// See: <https://sqlite.org/stricttables.html>
pub const ANY: TypeMarker = TypeMarker;

/// Stores a `bool` as `INTEGER` 0 or 1.
///
/// # Examples
/// ```rust
/// # let _ = r####"
/// #[column(boolean)]
/// active: bool,
/// # "####;
/// ```
///
/// `SQLite` has no native BOOLEAN. Values are stored as INTEGER (0 for false, 1 for true).
///
/// See: <https://sqlite.org/datatype3.html#boolean_datatype>
pub const BOOLEAN: TypeMarker = TypeMarker;
