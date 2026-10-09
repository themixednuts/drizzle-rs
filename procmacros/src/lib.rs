//! Procedural macros for Drizzle RS: schema definitions, row mapping and SQL
//! templates.
//!
//! Use these macros through the `drizzle` crate, not directly. Each dialect's
//! prelude (`drizzle::sqlite::prelude`, `drizzle::postgres::prelude`,
//! `drizzle::mysql::prelude`) re-exports its macros together with the
//! attribute markers they accept. The generated code names items by their
//! `drizzle::...` paths, so `drizzle` must be a dependency of the crate that
//! uses the macros.
//!
//! Each dialect needs its cargo feature (`sqlite`, `postgres` or `mysql`).
//! `SQLite` and `PostgreSQL` row conversions are generated per driver, so
//! enable the driver feature too (such as `rusqlite` or `tokio-postgres`).
//! `MySQL` row conversions are driver-neutral.
//!
//! # Macros
//!
//! | Purpose | `SQLite` | `PostgreSQL` | `MySQL` |
//! |---|---|---|---|
//! | Table | [`SQLiteTable`] | [`PostgresTable`] | [`MySQLTable`] |
//! | View | [`SQLiteView`] | [`PostgresView`] | [`MySQLView`] |
//! | Index | [`SQLiteIndex`] | [`PostgresIndex`] | [`MySQLIndex`] |
//! | Enum column type | [`SQLiteEnum`] | [`PostgresEnum`] | [`MySQLEnum`] |
//! | Schema (all objects) | [`SQLiteSchema`] | [`PostgresSchema`] | [`MySQLSchema`] |
//! | Row struct for custom selects | [`SQLiteFromRow`] | [`PostgresFromRow`] | [`MySQLFromRow`] |
//! | Row-level security policy | | [`PostgresPolicy`] | |
//!
//! Dialect-neutral macros:
//!
//! - [`sql!`] builds a raw SQL fragment with embedded expressions.
//! - [`include_migrations!`] embeds a migrations folder at compile time.
//! - [`test`] (`#[drizzle::test]`) is internal to drizzle's own test suite.
//!
//! Top-level table, view and column attribute keys are case-insensitive:
//! `#[column(primary)]` and `#[column(PRIMARY)]` mean the same thing. Write
//! index attributes, the arguments nested inside `foreign_key(...)`,
//! `unique(...)` and `check(...)`, and the view `query(...)` DSL in
//! lowercase.
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "sqlite")]
//! # fn main() {
//! use drizzle::core::expr::eq;
//! use drizzle::sqlite::builder::QueryBuilder;
//! use drizzle::sqlite::prelude::*;
//!
//! #[SQLiteTable(name = "users")]
//! struct Users {
//!     #[column(primary)]
//!     id: i64,
//!     name: String,
//!     email: Option<String>,
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct Schema {
//!     users: Users,
//! }
//!
//! let Schema { users } = Schema::new();
//! let query = QueryBuilder::new::<Schema>()
//!     .select(users.name)
//!     .from(users)
//!     .r#where(eq(users.id, 1));
//!
//! assert_eq!(
//!     query.to_sql().sql(),
//!     r#"SELECT "users"."name" FROM "users" WHERE "users"."id" = ?"#
//! );
//!
//! // A driver's `Drizzle::new(conn)` returns the same schema handles and runs
//! // these queries; `db.create()` runs `Schema`'s CREATE statements.
//! # }
//! # #[cfg(not(feature = "sqlite"))]
//! # fn main() {}
//! ```

// Rationale for crate-level clippy allow:
//   `too_many_lines`: this crate is dominated by proc-macro code-generation
//   functions built around a single cohesive `quote!` expansion. The line
//   count reflects the size of the generated code, not the algorithmic
//   complexity of the function — splitting them produces indirection without
//   reducing cognitive load. The 100-line clippy default is a poor proxy here.
#![allow(clippy::too_many_lines)]
// Built with no dialect (for example as a dependency of `drizzle` with default
// features), every dialect module is compiled out and the shared helpers they
// use go unused.
#![cfg_attr(
    not(any(feature = "sqlite", feature = "postgres", feature = "mysql")),
    allow(dead_code, unused_imports)
)]

extern crate proc_macro;

mod common;
mod drizzle_test;
mod fromrow;
mod generators;

mod migrations;
mod paths;
mod sql;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "mysql")]
mod mysql;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Derive a `SQLite` column type for a fieldless enum.
///
/// The enum decides its own storage:
///
/// | Enum shape | Storage | Stored value |
/// |---|---|---|
/// | Any variant has an explicit discriminant (`High = 10`), or the enum has an integer `#[repr]` | `INTEGER` | the discriminant |
/// | Otherwise | `TEXT` | the variant name, exactly as written |
///
/// Mark the column with `#[column(enum)]` in a [`SQLiteTable`]. The column
/// attribute does not pick the storage. An `integer` or `text` marker next to
/// `enum` only restates it, and the table fails to compile if it disagrees.
///
/// The derive takes no attributes. It needs at least one variant, and every
/// variant must be a unit variant.
///
/// # Generated impls
///
/// - `Display`, `FromStr`, `AsRef<str>`, and `TryFrom<&str>` / `TryFrom<String>`,
///   all using the variant name.
/// - `From<Enum> for i64` and `TryFrom<i64>`, using the discriminant.
///   Variants without an explicit discriminant count up from the previous
///   one, as in Rust; duplicate values are a compile error.
/// - The `SQLite` column, value and row conversions (`ToSQL`, `TryFrom<SQLiteValue>`,
///   and conversions for each enabled driver), so the enum works as a column
///   type, a bound parameter, and a selected value.
///
/// # Examples
///
/// Text storage:
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[derive(SQLiteEnum, Clone, Copy, Debug, PartialEq)]
/// enum Role {
///     Member,
///     Admin,
/// }
///
/// #[SQLiteTable]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     #[column(enum)]
///     role: Role,
/// }
///
/// assert_eq!(Role::Admin.to_string(), "Admin");
/// assert_eq!("Member".parse::<Role>().unwrap(), Role::Member);
/// assert!(Users::ddl_sql().contains("`role` TEXT NOT NULL"));
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
///
/// Integer storage, chosen by the explicit discriminants:
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[derive(SQLiteEnum, Clone, Copy, Debug, PartialEq)]
/// enum Priority {
///     Low = 1,
///     Medium = 5,
///     High = 10,
/// }
///
/// #[SQLiteTable]
/// struct Tasks {
///     #[column(primary)]
///     id: i64,
///     #[column(enum)]
///     priority: Priority,
/// }
///
/// assert_eq!(i64::from(Priority::High), 10);
/// assert_eq!(Priority::try_from(5_i64).unwrap(), Priority::Medium);
/// assert!(Tasks::ddl_sql().contains("`priority` INTEGER NOT NULL"));
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteEnum)]
pub fn sqlite_enum_derive(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{Data, DeriveInput, parse_macro_input};

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Check if this is an enum or tuple struct
    match &input.data {
        Data::Enum(data) => {
            // Check if the enum has any variants
            if data.variants.is_empty() {
                return quote! {
                    compile_error!("#[derive(SQLiteEnum)] requires at least one variant");
                }
                .into();
            }

            // Generate implementation for enum
            match crate::sqlite::r#enum::generate_enum_impl(name, data, &input.attrs) {
                Ok(ts) => ts.into(),
                Err(e) => e.to_compile_error().into(),
            }
        }
        _ => quote! {
            compile_error!("#[derive(SQLiteEnum)] can only be applied to enums");
        }
        .into(),
    }
}

/// Define a `SQLite` table from a struct.
///
/// Each named field becomes a column. The macro replaces the struct with a
/// zero-sized table handle whose fields are column handles, and generates the
/// models used to insert, select and update rows. See
/// [CREATE TABLE](https://sqlite.org/lang_createtable.html) for the SQL side.
///
/// # Table attributes
///
/// Written as `#[SQLiteTable(...)]`. All are optional.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "users"` | SQL table name. Defaults to the struct name in `snake_case` (`UserAccount` becomes `user_account`). |
/// | `strict` | Create a [STRICT table](https://sqlite.org/stricttables.html). Every column must then use `INTEGER`, `REAL`, `TEXT`, `BLOB` or `ANY`. |
/// | `without_rowid` | Create a [WITHOUT ROWID table](https://sqlite.org/withoutrowid.html). Needs a primary key and rules out `autoincrement`. |
/// | `unique(a, b)` or `unique(columns(a, b), name = "...")` | Table-level `UNIQUE` constraint over the named fields. |
/// | `check(expr = "a < b", name = "...")` | Table-level `CHECK` constraint. The expression is raw SQL. |
/// | `foreign_key(columns(a, b), references(Parent, x, y), on_delete = "CASCADE", on_update = "...", relation = "...", many_to_many = "...")` | Composite foreign key from fields `a, b` to `Parent`'s fields `x, y`. The actions are SQL text, such as `"SET NULL"`. `relation` and `many_to_many` name its relations (see [Relations](#relations)). |
///
/// # Column attributes
///
/// Written as `#[column(...)]` on a field. All are optional.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `primary` (or `primary_key`) | Primary key. Put it on several fields for a composite key. |
/// | `autoincrement` | `AUTOINCREMENT`. Only on a single-column `INTEGER` primary key, and not with `without_rowid`. |
/// | `unique` | `UNIQUE` constraint. |
/// | `integer`, `text`, `real`, `blob`, `numeric`, `boolean`, `any` | Override the SQL type inferred from the Rust type (`boolean` stores `INTEGER` 0/1). `any` is allowed only in `strict` tables. |
/// | `enum` | The field's type derives [`SQLiteEnum`]; the enum decides `TEXT` or `INTEGER` storage. |
/// | `json` | Store a `serde` type as JSON `TEXT` (needs the `serde` feature). |
/// | `name = "col"` | SQL column name. Defaults to the field name in `snake_case`. |
/// | `default = value` | SQL `DEFAULT` clause. A string literal becomes a SQL string and `true`/`false` become `1`/`0`. A path or call is written as SQL in parentheses (`CURRENT_TIME`, `CURRENT_DATE` and `CURRENT_TIMESTAMP` stay bare). |
/// | `default_fn = path` | Rust function called to fill the field when an insert model is created. Cannot be combined with `default`. |
/// | `references = Table::column` | Foreign key to another table's column. |
/// | `on_delete = ACTION`, `on_update = ACTION` | Referential action for `references`: `CASCADE`, `SET_NULL`, `SET_DEFAULT`, `RESTRICT` or `NO_ACTION`. |
/// | `relation = "name"` | Name of the reverse accessor the referenced table gets through this column (see [Relations](#relations)). Needs `references`. |
/// | `many_to_many = "name"` | Name of the many-to-many accessor the referenced table gets through this link table; also makes a table with two foreign keys a link (see [Relations](#relations)). Needs `references`. |
/// | `collate = NOCASE` | Column collation: `BINARY`, `NOCASE`, `RTRIM`, or any name as a string. |
/// | `check = "score >= 0"` | Column-level `CHECK` constraint. The expression is raw SQL. |
/// | `generated(stored, "expr")` or `generated(virtual, "expr")` | Generated column. Never written by inserts. Cannot be combined with `default`. |
///
/// The SQL type comes from the Rust type when no type attribute is given:
/// integers and `bool` map to `INTEGER`, floats to `REAL`, `String` and the
/// date/time types to `TEXT`, and `Vec<u8>` and `uuid::Uuid` to `BLOB`. Any
/// other type must implement `DrizzleSQLiteColumn`, which supplies its SQL
/// type.
///
/// # Nullability
///
/// `Option<T>` makes a column nullable. Every other field gets `NOT NULL`.
/// There is no `not_null` attribute.
///
/// # Generated items
///
/// For `#[SQLiteTable] struct Users { ... }` the macro generates:
///
/// | Item | Purpose |
/// |---|---|
/// | `Users` | The table handle. Its fields (`users.name`) are typed columns for building queries. `Users::TABLE_NAME` is the SQL name, and `Users::ddl_sql()` returns the `CREATE TABLE` statement. |
/// | `SelectUsers` | One row: every column, with `Option<T>` for nullable ones. |
/// | `PartialSelectUsers` | A row where every field is `Option<T>`, for partial selects. |
/// | `InsertUsers` | Insert builder. `InsertUsers::new(...)` takes the required columns in field order; `with_<field>(value)` sets the rest. |
/// | `UpdateUsers` | Update builder. Start from `UpdateUsers::default()` and set columns with `with_<field>(value)`. |
/// | `users` module | One type per column (`users::Name` is the type of `Users::name`), plus the alias column types. |
/// | `Users::alias::<Tag>()` | A second, independently named copy of the table for self-joins (name the alias with a type made by the prelude's `tag!` macro). |
///
/// A column is required in `InsertUsers::new` unless it is nullable, has a
/// `default`, `default_fn` or `generated` value, or is an `INTEGER` primary
/// key of a rowid table (which `SQLite` fills from the rowid).
///
/// Row conversions for `SelectUsers` and `PartialSelectUsers` are generated
/// for every enabled driver (`rusqlite`, `libsql`, `turso`).
///
/// # Relations
///
/// With the `query` feature, `references` also generates relation accessors
/// for the relational query API:
///
/// - Forward, on this table: the column name without its `_id` suffix
///   (`author_id` gives `posts.author()`). A nullable key loads an `Option`.
/// - Reverse, on the referenced table: this struct's name in plural
///   `snake_case`, after the column's role. A column named after the table it
///   references has none (`user_id` gives `users.posts()`); any other name is
///   one (`author_id` gives `users.author_posts()`, `parent_id` on a
///   self-reference gives `categories.parent_categories()`).
/// - One-to-one: when the column alone is unique or the primary key, the
///   reverse accessor takes the singular and loads an `Option`
///   (`users.profile()`).
/// - Many-to-many: a link table, whose rows are a pair of foreign keys, gives
///   each side an accessor to the other, named after the other column
///   (`posts.tags()` and `tags.posts()` through `PostTags`). The pair is its
///   primary key or a `UNIQUE` constraint, or it has no other column except a
///   single-column primary key, and neither key is unique alone. A link whose
///   name adds to what it links appends it (`users.posts_via_likes()`
///   through `PostLikes`), so two links between the same tables never clash.
/// - A table-level `foreign_key(...)` gives the same relations; its forward
///   accessor is the referenced struct's singular name.
/// - `relation = "name"` and `many_to_many = "name"` choose other names, on
///   the column or inside `foreign_key(...)`. The macro reports two accessors
///   it generates with one name; two tables that give a third the same name
///   get a duplicate definition from rustc at both columns.
///
/// # Examples
///
/// A table and the SQL it creates:
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     #[column(unique, collate = NOCASE)]
///     email: String,
///     #[column(default = "member")]
///     role: String,
///     age: Option<i64>,
/// }
///
/// assert_eq!(Users::TABLE_NAME, "users");
/// assert_eq!(
///     Users::ddl_sql(),
///     "CREATE TABLE `users` (\n\
///     \t`id` INTEGER PRIMARY KEY,\n\
///     \t`email` TEXT NOT NULL UNIQUE COLLATE NOCASE,\n\
///     \t`role` TEXT DEFAULT 'member' NOT NULL,\n\
///     \t`age` INTEGER\n\
///     );"
/// );
///
/// // `id` is filled by SQLite and `role` has a default, so only `email` is required.
/// let alice = InsertUsers::new("alice@example.com").with_age(30);
/// # let _ = alice;
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
///
/// Foreign keys, generated columns and table-level constraints:
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     name: String,
/// }
///
/// #[SQLiteTable(strict, check(name = "title_not_empty", expr = "title <> ''"))]
/// struct Posts {
///     #[column(primary, autoincrement)]
///     id: i64,
///     #[column(references = Users::id, on_delete = CASCADE)]
///     author_id: i64,
///     title: String,
///     #[column(generated(virtual, "length(title)"))]
///     title_length: i64,
/// }
///
/// assert_eq!(
///     Posts::ddl_sql(),
///     "CREATE TABLE `posts` (\n\
///     \t`id` INTEGER PRIMARY KEY AUTOINCREMENT,\n\
///     \t`author_id` INTEGER NOT NULL,\n\
///     \t`title` TEXT NOT NULL,\n\
///     \t`title_length` INTEGER GENERATED ALWAYS AS (length(title)) VIRTUAL NOT NULL,\n\
///     \tCONSTRAINT `fk_posts_author_id_users_id_fk` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`) ON DELETE CASCADE,\n\
///     \tCONSTRAINT `title_not_empty` CHECK(title <> '')\n\
///     ) STRICT;"
/// );
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
///
/// Enum and JSON columns:
///
/// ```rust
/// # #[cfg(all(feature = "sqlite", feature = "serde"))]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(SQLiteEnum, Clone, Copy, Debug, PartialEq)]
/// enum Role {
///     Member,
///     Admin,
/// }
///
/// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// struct Settings {
///     theme: String,
/// }
///
/// #[SQLiteTable]
/// struct Accounts {
///     #[column(primary)]
///     id: i64,
///     #[column(enum)]
///     role: Role,
///     #[column(json)]
///     settings: Option<Settings>,
/// }
///
/// let account = InsertAccounts::new(Role::Admin).with_settings(Settings { theme: "dark".into() });
/// # let _ = account;
/// # }
/// # #[cfg(not(all(feature = "sqlite", feature = "serde")))]
/// # fn main() {}
/// ```
///
/// # Compile-time checks
///
/// The insert builder only accepts complete rows. Leaving out a required
/// column is a type error:
///
/// ```rust,compile_fail
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     name: String,
///     email: String,
/// }
///
/// // `email` is required too.
/// let user = InsertUsers::new("Alice");
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() { compile_error!("needs sqlite") }
/// ```
///
/// The macro also rejects, among others: `autoincrement` on a non-`INTEGER`
/// or non-primary column, `autoincrement` with `without_rowid`, column types
/// that `strict` does not allow, `on_delete`, `on_update`, `relation` or
/// `many_to_many` without `references`, and a `default` whose literal does
/// not fit the column type.
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::sqlite::table::TableAttributes);

    match crate::sqlite::table::table_attr_macro(&input, &attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `SQLite` view from a struct.
///
/// Each named field is a column of the view. The view is queried like a
/// table: the macro generates the same column handles, `Select*` models and
/// row conversions as [`SQLiteTable`]. Field attributes are the
/// `#[column(...)]` attributes of [`SQLiteTable`]; usually none are needed.
///
/// # Attributes
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "active_users"` | SQL view name. Defaults to the struct name in `snake_case`. |
/// | `definition = "SELECT ..."` | The view's query as raw SQL. |
/// | `definition = expr` | The view's query as a query-builder expression, rendered when the view is created. |
/// | `query(...)` | The view's query in a small typed DSL, rendered to SQL at compile time (see below). |
/// | `existing` | The view already exists in the database: drizzle queries it but never creates it. |
///
/// A view needs one of `definition` or `query(...)` unless it is `existing`.
/// `definition` and `query(...)` cannot be combined.
///
/// # The `query(...)` DSL
///
/// Clauses, in any order, each at most once (joins may repeat):
///
/// - `select(Table::col, count(Table::col), ...)` and `from(Table)` (required)
/// - `join(Table, cond)` (or `inner_join`), `left_join`, `right_join`,
///   `full_join`, `cross_join(Table)`
/// - `filter(cond)`, `group_by(Table::col, ...)`, `having(cond)`
/// - `order_by(asc(Table::col), desc(Table::col), ...)`, `limit(n)`, `offset(n)`
///
/// Conditions use `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `not_like`,
/// `is_null`, `is_not_null`, `between`, `not_between`,
/// `in_array(Table::col, [a, b])`, `and`, `or` and `not`. Aggregates are
/// `count`, `count_all()`, `count_distinct`, `sum`, `avg`, `min` and `max`.
/// Columns are written `Table::column`, and their types are checked against
/// the compared values. Literals are written into the SQL, not bound.
///
/// `select(...)` must list one item per struct field, in field order; each
/// item is aliased to its field's column name.
///
/// # Generated items
///
/// Besides the table-like items, the view gets `VIEW_NAME`,
/// `VIEW_DEFINITION_SQL` (empty for an expression `definition`), and
/// `ddl_sql()`, which returns the `CREATE VIEW` statement.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     email: String,
///     active: bool,
/// }
///
/// #[SQLiteView(query(
///     select(Users::id, Users::email),
///     from(Users),
///     filter(eq(Users::active, true)),
/// ))]
/// struct ActiveUsers {
///     id: i64,
///     email: String,
/// }
///
/// #[SQLiteView(name = "user_ids", definition = "SELECT id FROM users")]
/// struct UserIds {
///     id: i64,
/// }
///
/// assert_eq!(
///     ActiveUsers::ddl_sql(),
///     r#"CREATE VIEW "active_users" AS SELECT "users"."id" AS "id", "users"."email" AS "email" FROM "users" WHERE "users"."active" = 1"#
/// );
/// assert_eq!(UserIds::ddl_sql(), r#"CREATE VIEW "user_ids" AS SELECT id FROM users"#);
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteView(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::sqlite::view::ViewAttributes);

    match crate::sqlite::view::view_attr_macro(&input, &attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `SQLite` index on one or more columns of a table.
///
/// Apply it to a tuple struct whose fields name the indexed columns as
/// `Table::column`. All columns must belong to the same table. Add the index
/// to a [`SQLiteSchema`] so `db.create()` and migrations include it.
///
/// # Attributes
///
/// | Attribute | Meaning |
/// |---|---|
/// | `unique` | Create a `UNIQUE` index. |
/// | `name = "..."` | SQL index name. Defaults to the struct name lowercased, with `_` before each inner capital (`UsersEmailIdx` becomes `users_email_idx`). |
/// | `where = "..."` | Partial index predicate, as raw SQL. Column names inside the string are not checked. |
///
/// # Generated items
///
/// The struct becomes a unit struct with `new()`, and `ddl_sql()` returns its
/// `CREATE INDEX` statement. A `unique` index can also be passed to an
/// insert's `on_conflict(...)` as the conflict target.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable(name = "posts")]
/// struct Posts {
///     #[column(primary)]
///     id: i64,
///     slug: String,
///     author_id: i64,
///     title: String,
/// }
///
/// #[SQLiteIndex(unique)]
/// struct PostsSlugIdx(Posts::slug);
///
/// #[SQLiteIndex(name = "posts_by_author", where = "title <> ''")]
/// struct PostsAuthorIdx(Posts::author_id, Posts::title);
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     posts: Posts,
///     posts_slug_idx: PostsSlugIdx,
///     posts_author_idx: PostsAuthorIdx,
/// }
///
/// assert_eq!(
///     PostsSlugIdx::ddl_sql(),
///     r#"CREATE UNIQUE INDEX "posts_slug_idx" ON "posts" ("slug")"#
/// );
/// assert_eq!(
///     PostsAuthorIdx::ddl_sql(),
///     r#"CREATE INDEX "posts_by_author" ON "posts" ("author_id", "title") WHERE title <> ''"#
/// );
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteIndex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_input = syn::parse_macro_input!(attr as crate::sqlite::index::IndexAttributes);

    match crate::sqlite::index::sqlite_index_attr_macro(attr_input, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive row decoding for a custom result struct on `SQLite` drivers.
///
/// Use it for query results that are not a whole table row, such as a join
/// or a few chosen columns. For each enabled driver (`rusqlite`, `libsql`,
/// `turso`) the struct gets `TryFrom<&Row>` and the row-decoding traits the
/// query builder uses, so `.all()` and `.get()` can return it.
///
/// # Attributes
///
/// | Attribute | Where | Meaning |
/// |---|---|---|
/// | `#[from(Table)]` | struct | Default table for fields without `#[column(...)]`: field `name` reads `Table::name`. |
/// | `#[column(Table::field)]` | field | Read this field from `Table::field`, whatever the field is called. |
///
/// # Selecting into the struct
///
/// A struct with named fields also gets a selector, `Struct::Select`. Pass it
/// to `select(...)` and it selects each field's column, aliased to the field
/// name. The query is checked against it when it runs: every table the
/// struct reads must be in the query, and a field read from an outer-joined
/// table must be an `Option<T>`.
///
/// Without `#[from]` or `#[column]`, the selector selects the bare field
/// names (`"name"`), which suits single-table queries and views.
///
/// # Decoding
///
/// - When the query builder decodes a row, named fields are read by column
///   name with `rusqlite` and `libsql` if the struct uses no `#[from]` or
///   `#[column]`, and by position otherwise. `turso` rows carry no column
///   names, so `turso` always reads by position.
/// - `TryFrom<&Row>` reads named fields by name with `rusqlite` and
///   `libsql`, and by position with `turso`.
/// - Tuple structs decode by position (field 0 is column 0) and get no
///   selector.
/// - `Option<T>` fields accept `NULL`.
/// - Each field type must be readable by the driver: `rusqlite` uses its own
///   `FromSql` trait, `libsql` and `turso` use drizzle's `SQLite` value
///   conversion. Integers, floats, `bool` (`0` is false, anything else
///   true), `String`, `Vec<u8>` and [`SQLiteEnum`] types work with all three.
///
/// The derive reads each value directly, so table-owned JSON codecs do not
/// run. Select into the table's `Select*` model when you need them.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::core::expr::eq;
/// use drizzle::sqlite::builder::QueryBuilder;
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     name: String,
/// }
///
/// #[SQLiteTable(name = "posts")]
/// struct Posts {
///     #[column(primary)]
///     id: i64,
///     #[column(references = Users::id)]
///     author_id: i64,
///     title: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     users: Users,
///     posts: Posts,
/// }
///
/// #[derive(SQLiteFromRow, Debug)]
/// #[from(Users)]
/// struct UserWithPost {
///     name: String,
///     // Posts is left-joined, so its fields are optional.
///     #[column(Posts::title)]
///     post_title: Option<String>,
/// }
///
/// let Schema { users, posts } = Schema::new();
/// let query = QueryBuilder::new::<Schema>()
///     .select(UserWithPost::Select)
///     .from(users)
///     .left_join((posts, eq(users.id, posts.author_id)));
///
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."name" AS "name", "posts"."title" AS "post_title" FROM "users" LEFT JOIN "posts" ON "users"."id" = "posts"."author_id""#
/// );
///
/// // With a driver: `let rows: Vec<UserWithPost> = db.select(UserWithPost::Select)...all()?;`
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
///
/// A tuple struct for a single value:
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[derive(SQLiteFromRow)]
/// struct Count(i64);
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteFromRow, attributes(column, from))]
pub fn sqlite_from_row_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::fromrow::generate_sqlite_from_row_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive row decoding for a custom result struct on `PostgreSQL` drivers.
///
/// The `PostgreSQL` counterpart of [`SQLiteFromRow`], with the same
/// attributes and the same `Struct::Select` selector:
///
/// | Attribute | Where | Meaning |
/// |---|---|---|
/// | `#[from(Table)]` | struct | Default table for fields without `#[column(...)]`. |
/// | `#[column(Table::field)]` | field | Read this field from `Table::field`. |
///
/// With `postgres-sync` or `tokio-postgres` the struct gets `TryFrom<&Row>`
/// and the row-decoding traits the query builder uses, for the one row type
/// the two drivers share. No conversion is generated for `aws-data-api`
/// rows.
///
/// When the query builder decodes a row, named fields are read by column
/// name if no field uses `#[column]` (`#[from]` alone keeps name lookup) and
/// the struct starts at the row's first column, and by position otherwise.
/// `TryFrom<&Row>` always reads named fields by name. Tuple structs decode by
/// position.
///
/// # Panics
///
/// Most field types are read with the driver's `Row::get`, which panics
/// instead of returning an error when the column is missing or has an
/// incompatible type.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// use drizzle::postgres::builder::QueryBuilder;
/// use drizzle::postgres::prelude::*;
///
/// #[PostgresTable(name = "users")]
/// struct Users {
///     #[column(serial, primary)]
///     id: i32,
///     name: String,
///     email: Option<String>,
/// }
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     users: Users,
/// }
///
/// #[derive(PostgresFromRow, Debug)]
/// #[from(Users)]
/// struct Contact {
///     name: String,
///     email: Option<String>,
/// }
///
/// let Schema { users } = Schema::new();
/// let query = QueryBuilder::new::<Schema>().select(Contact::Select).from(users);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."name" AS "name", "users"."email" AS "email" FROM "users""#
/// );
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[proc_macro_derive(PostgresFromRow, attributes(column, from))]
pub fn postgres_from_row_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::fromrow::generate_postgres_from_row_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive a `SQLite` schema: the set of tables, indexes and views an
/// application uses.
///
/// Apply it to a struct with named fields, one per table ([`SQLiteTable`]),
/// index ([`SQLiteIndex`]) or view ([`SQLiteView`]). The field names are up to
/// you. A driver's `Drizzle::new(conn)` returns the schema next to the
/// connection, and the schema is what `db.create()` and migrations work from.
///
/// The derive takes no attributes.
///
/// # Generated items
///
/// - `Schema::new()` (a `const fn`) and `Default`, building every handle.
/// - `Clone`, `Copy` and `Debug`. Do not derive these (or `Default`) yourself:
///   the macro reports an error if they appear in a `#[derive]` on the struct.
/// - `schema.items()`, a tuple of references to every field, and
///   `From<Schema>` for the tuple of fields.
/// - `SQLSchemaImpl`, whose `create_statements()` returns the statements a
///   migration from an empty database runs: each table followed by its
///   indexes, then views. SQLite checks a foreign key when rows change, not
///   when a table is created, so tables that reference each other need no
///   particular order.
/// - The migrations `Schema` trait, so the CLI and `migrate()` can snapshot it.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() -> drizzle::Result<()> {
/// use drizzle::core::SQLSchemaImpl;
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     email: String,
/// }
///
/// #[SQLiteTable(name = "posts")]
/// struct Posts {
///     #[column(primary)]
///     id: i64,
///     #[column(references = Users::id)]
///     author_id: i64,
/// }
///
/// #[SQLiteIndex(unique)]
/// struct UsersEmailIdx(Users::email);
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     posts: Posts,
///     users: Users,
///     users_email_idx: UsersEmailIdx,
/// }
///
/// let statements: Vec<String> = Schema::new().create_statements()?.collect();
/// let position = |prefix: &str| {
///     statements
///         .iter()
///         .position(|sql| sql.starts_with(prefix))
///         .expect(prefix)
/// };
/// // Each index follows its table.
/// assert!(position("CREATE TABLE `users`") < position("CREATE UNIQUE INDEX `users_email_idx`"));
/// assert!(position("CREATE TABLE `posts`") < statements.len());
///
/// // Destructure to get the handles used in queries.
/// let Schema { users, posts, .. } = Schema::new();
/// # let _ = (users, posts);
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
///
/// # Compile-time checks
///
/// Every table a foreign key points to must be in the schema:
///
/// ```rust,compile_fail
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Users {
///     #[column(primary)]
///     id: i64,
/// }
///
/// #[SQLiteTable]
/// struct Posts {
///     #[column(primary)]
///     id: i64,
///     #[column(references = Users::id)]
///     author_id: i64,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     posts: Posts, // error: `Users` is referenced but missing
/// }
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() { compile_error!("needs sqlite") }
/// ```
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteSchema)]
pub fn sqlite_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::sqlite::schema::generate_sqlite_schema_derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive a `PostgreSQL` schema: the set of database objects an application
/// uses.
///
/// Apply it to a struct with named fields, one per table
/// ([`PostgresTable`]), index ([`PostgresIndex`]), view ([`PostgresView`]),
/// native enum ([`PostgresEnum`]) or policy ([`PostgresPolicy`]). A driver's
/// `Drizzle::new(client)` returns the schema next to the connection, and the
/// schema is what `db.create()` and migrations work from. Every field type
/// must implement `Default` (derive it on enums).
///
/// The derive takes no attributes.
///
/// # Generated items
///
/// - `Schema::new()` and `Default`, building every handle.
/// - `Clone`, `Copy` and `Debug`. Do not derive these (or `Default`) yourself:
///   the macro reports an error if they appear in a `#[derive]` on the struct.
/// - `schema.items()` and `From<Schema>` for the tuple of fields.
/// - `SQLSchemaImpl`, whose `create_statements()` returns the statements a
///   migration from an empty database runs: schemas and enum types first,
///   then tables (referenced tables first) with their comments, indexes,
///   row-level security and policies, then views. A foreign key that closes a
///   cycle is added once both of its tables exist.
/// - The migrations `Schema` trait, so the CLI and `migrate()` can snapshot it.
///
/// As with [`SQLiteSchema`], every table a foreign key points to must be in
/// the schema, or the derive fails to compile.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() -> drizzle::Result<()> {
/// use drizzle::core::SQLSchemaImpl;
/// use drizzle::postgres::prelude::*;
///
/// #[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
/// enum Status {
///     #[default]
///     Open,
///     Closed,
/// }
///
/// #[PostgresTable(name = "tickets")]
/// struct Tickets {
///     #[column(serial, primary)]
///     id: i32,
///     #[column(enum)]
///     status: Status,
/// }
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     status: Status,
///     tickets: Tickets,
/// }
///
/// let statements: Vec<String> = Schema::new().create_statements()?.collect();
/// assert_eq!(statements[0], r#"CREATE TYPE "Status" AS ENUM ('Open', 'Closed');"#);
/// assert!(statements[1].starts_with(r#"CREATE TABLE "tickets""#));
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[proc_macro_derive(PostgresSchema)]
pub fn postgres_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::postgres::generate_postgres_schema_derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Build a SQL fragment from a template with embedded expressions.
///
/// Text in the template is copied as raw SQL. Each `{...}` is a Rust
/// expression that implements `ToSQL`: a table renders as its quoted name, a
/// column as its qualified name, and a plain value becomes a bound parameter
/// (`?` or `$n`), never inlined text. The result is a `SQL` value for the
/// dialect the expressions belong to.
///
/// Two forms:
///
/// - Named: `sql!("SELECT * FROM {users} WHERE {users.id} = {id}")`. Any
///   expression can go inside the braces.
/// - Positional: `sql!("SELECT * FROM {} WHERE {} = {}", users, users.id, 42)`.
///   Each empty `{}` takes the next argument. The number of `{}` must match
///   the number of arguments.
///
/// Write `{{` and `}}` for literal braces. An unmatched brace or an argument
/// count mismatch is a compile error.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use drizzle::sql;
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[column(primary)]
///     id: i64,
///     email: String,
/// }
///
/// let users = Users::default();
///
/// let named = sql!("SELECT {users.email} FROM {users} WHERE {users.id} = {42}");
/// assert_eq!(
///     named.sql(),
///     r#"SELECT "users"."email" FROM "users" WHERE "users"."id" = ?"#
/// );
///
/// let positional = sql!("SELECT * FROM {} WHERE {} = {}", users, users.id, 42);
/// assert_eq!(positional.sql(), r#"SELECT * FROM "users" WHERE "users"."id" = ?"#);
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
#[proc_macro]
pub fn sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as crate::sql::SqlInput);

    match crate::sql::sql_impl(input) {
        Ok(output) => output.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Embed a migrations folder in the binary and return `Vec<Migration>`.
///
/// The argument is a string literal: the path to the folder that
/// `drizzle generate` writes, relative to the crate's `Cargo.toml`. Each
/// `<timestamp>_<name>/migration.sql` inside becomes one `Migration`, sorted
/// by folder name, with its SQL split into statements at compile time. Pass
/// the result to a driver's `migrate` to apply the ones not yet run.
///
/// # Examples
///
/// ```rust
/// let migrations: Vec<drizzle::migrations::Migration> = drizzle::include_migrations!("./drizzle");
/// # // Embeds procmacros/drizzle, this doctest's one-migration fixture.
/// # assert_eq!(migrations.len(), 1);
/// ```
///
/// # Errors
///
/// A missing or unreadable folder is a compile error, not an empty list. So
/// is a folder in the old drizzle-kit layout (`meta/_journal.json`; run
/// `drizzle up` to convert it) and a migration folder with a `snapshot.json`
/// but no `migration.sql`.
///
/// # Rebuilding after `drizzle generate`
///
/// Every embedded `migration.sql` is tracked, so editing one rebuilds the
/// crate. A *new* migration folder is not: a procedural macro cannot ask the
/// compiler to watch a directory. Add a build script whose `main` prints
/// this line, so that generating a migration rebuilds the crate that embeds
/// them:
///
/// ```rust
/// // In build.rs, inside `fn main`:
/// println!("cargo:rerun-if-changed=drizzle");
/// ```
#[proc_macro]
pub fn include_migrations(input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    match crate::migrations::include_migrations_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Run one test body against every enabled database driver.
///
/// **Internal to drizzle's own test suite.** The expansion calls helpers in
/// that suite's `crate::common::helpers` module (connection setup, `TestDb`,
/// the panic hook), so it does not compile in other crates. It is exported
/// only because the suite is a separate crate.
///
/// The macro turns one synchronous `fn` into a test module per driver of the
/// chosen dialect, each gated on that driver's feature: `rusqlite`, `libsql`
/// and `turso` for `SQLite`; `postgres-sync` and `tokio-postgres` for
/// `PostgreSQL`; `mysql-sync` and `mysql-async` for `MySQL`. Async drivers get
/// `.await` added to terminal calls (`.execute()`, `.all()`, `.get()`,
/// transactions, and so on). A failing terminal call panics with the SQL
/// that ran.
///
/// # Arguments
///
/// | Form | Dialect |
/// |---|---|
/// | `#[drizzle::test]` | From the test file's path: a `sqlite`, `postgres` or `mysql` directory. Anything else is a compile error asking for an explicit dialect. |
/// | `#[drizzle::test(sqlite)]` | `SQLite` |
/// | `#[drizzle::test(postgres)]` | `PostgreSQL` |
/// | `#[drizzle::test(mysql)]` | `MySQL` |
///
/// # Signature
///
/// ```ignore
/// #[drizzle::test]
/// fn inserts_a_user(db: &mut TestDb<MySchema>) {
///     let MySchema { users } = schema;
///     db.insert(users).values([InsertUsers::new("Alice")]).execute();
/// }
/// ```
///
/// - The function must be synchronous, with no generics.
/// - Its only parameter must be named `db`, typed `&mut TestDb<S>`,
///   `&TestDb<S>`, or `TestDb<S>`. `S` must implement `SQLSchemaImpl`,
///   `Default` and `Copy` (any schema derive does).
/// - The body gets a `schema: S` local, and helper macros: `result!(expr)`
///   returns the call's `Result` instead of panicking on `Err`, `catch!`
///   expects a panic, and `next_row!` / `collect_rows!` read row cursors.
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    crate::drizzle_test::attribute_impl(args, item)
}

/// Derive a `PostgreSQL` column type for a fieldless enum.
///
/// The enum decides its own storage:
///
/// | Enum shape | Storage | Stored value |
/// |---|---|---|
/// | No integer `#[repr]` | native enum type, `CREATE TYPE "Name" AS ENUM (...)` | the variant name |
/// | Integer `#[repr]` (`#[repr(i32)]`, `#[repr(i16)]`, ...) | `integer` | the discriminant |
///
/// Mark the column with `#[column(enum)]` in a [`PostgresTable`]. A native
/// enum is a database object of its own: list it in the [`PostgresSchema`]
/// so its `CREATE TYPE` runs before the tables that use it. A schema field
/// must implement `Default`, so derive `Default` on enums you list there.
///
/// # Attributes
///
/// | Attribute | Meaning |
/// |---|---|
/// | `#[postgres_enum(schema = "app")]` | Create the native type in this schema instead of `public`. Not allowed with an integer `#[repr]`. |
///
/// The SQL type name is the enum's name, quoted (`CREATE TYPE "Mood" ...`),
/// so `PostgreSQL` keeps its case, and schema-qualified outside `public`
/// (`"app"."Mood"`) — the same spelling migrations generate. Columns refer
/// to it the same way. With an integer `#[repr]`, every discriminant must
/// fit in an `i32`.
///
/// # Generated impls
///
/// - `Display`, `FromStr`, `AsRef<str>`, and `TryFrom<&str>` / `TryFrom<String>`,
///   all using the variant name.
/// - `From<Enum> for i64` and `TryFrom` from the integer types, using the
///   discriminant.
/// - `new()`, which returns the first variant.
/// - The `PostgreSQL` column, value and row conversions, including
///   `postgres_types::ToSql` / `FromSql` when a driver feature is enabled.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// use drizzle::postgres::prelude::*;
///
/// #[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
/// enum Mood {
///     #[default]
///     Happy,
///     Sad,
/// }
///
/// #[derive(PostgresEnum, Clone, Copy, Debug, PartialEq)]
/// #[repr(i32)]
/// enum Priority {
///     Low = 1,
///     High = 10,
/// }
///
/// #[PostgresTable]
/// struct Entries {
///     #[column(serial, primary)]
///     id: i32,
///     #[column(enum)]
///     mood: Mood,
///     #[column(enum)]
///     priority: Priority,
/// }
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     mood: Mood,
///     entries: Entries,
/// }
///
/// assert_eq!(Mood::Sad.to_string(), "Sad");
/// assert_eq!(i64::from(Priority::High), 10);
/// assert!(Entries::ddl_sql().contains(r#""mood" "Mood" NOT NULL"#));
/// assert!(Entries::ddl_sql().contains(r#""priority" integer NOT NULL"#));
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[proc_macro_derive(PostgresEnum, attributes(postgres_enum))]
pub fn postgres_enum_derive(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{Data, DeriveInput, parse_macro_input};

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Check if this is an enum
    match &input.data {
        Data::Enum(data) => {
            // Check if the enum has any variants
            if data.variants.is_empty() {
                return quote! {
                    compile_error!("#[derive(PostgresEnum)] requires at least one variant");
                }
                .into();
            }

            // Generate implementation for enum
            match crate::postgres::r#enum::generate_enum_impl(name, data, &input.attrs) {
                Ok(ts) => ts.into(),
                Err(e) => e.to_compile_error().into(),
            }
        }
        _ => quote! {
            compile_error!("#[derive(PostgresEnum)] can only be applied to enums");
        }
        .into(),
    }
}

/// Define a `PostgreSQL` table from a struct.
///
/// Each named field becomes a column. The macro replaces the struct with a
/// zero-sized table handle whose fields are column handles, and generates the
/// models used to insert, select and update rows. See
/// [CREATE TABLE](https://www.postgresql.org/docs/current/sql-createtable.html)
/// for the SQL side.
///
/// # Table attributes
///
/// Written as `#[PostgresTable(...)]`. All are optional.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "users"` | SQL table name. Defaults to the struct name in `snake_case`. |
/// | `schema = "auth"` | Create the table in this schema. Defaults to `public`. `CREATE TABLE` names a `public` table without a schema prefix; index, policy and view-DSL SQL qualify it as `"public"."table"`. |
/// | `unlogged` | `CREATE UNLOGGED TABLE`: faster writes, not crash-safe. |
/// | `temporary` | `CREATE TEMPORARY TABLE`. |
/// | `inherits = "parent"` | Inherit from a parent table. |
/// | `tablespace = "name"` | Create the table in a tablespace. |
/// | `rls` | Enable row-level security (see [`PostgresPolicy`]). |
/// | `unique(a, b)` or `unique(columns(a, b), name = "...", nulls_not_distinct, deferrable, initially_deferred)` | Table-level `UNIQUE` constraint over the named fields. |
/// | `check(expr = "a < b", name = "...")` | Table-level `CHECK` constraint. The expression is raw SQL. |
/// | `foreign_key(columns(a, b), references(Parent, x, y), name = "...", on_delete = "...", on_update = "...", deferrable, initially_deferred, relation = "...", many_to_many = "...")` | Composite foreign key from fields `a, b` to `Parent`'s fields `x, y`. `name` defaults to `{table}_{a}_fkey`. `relation` and `many_to_many` name its relations (see [Relations](#relations)). |
/// | `primary_key(name = "...")` | Name the primary key formed by the `primary` fields (default `{table}_pkey`). |
///
/// # Column attributes
///
/// Written as `#[column(...)]` on a field. All are optional.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `primary` (or `primary_key`) | Primary key. Put it on several fields for a composite key. |
/// | `unique` | `UNIQUE` constraint. |
/// | `serial`, `smallserial`, `bigserial` | Auto-incrementing column. The field must be `i32`, `i16` or `i64` respectively. |
/// | `identity`, `identity(always)`, `identity(by_default)` | `GENERATED ... AS IDENTITY` (`always` when no mode is given). Sequence options may follow the mode: `identity(always, start = 100, increment = 10, min_value = 1, max_value = 1000, cache = 5, cycle)`. |
/// | `varchar(n)`, `char(n)` | `VARCHAR(n)` / `CHAR(n)` instead of `TEXT`, on a `String` field. |
/// | `enum` | The field's type derives [`PostgresEnum`]; the enum decides native enum or `integer` storage. |
/// | `json`, `jsonb` | Store a `serde` type as `JSON` / `JSONB` (needs the `serde` feature). |
/// | `name = "col"` | SQL column name. Defaults to the field name in `snake_case`. |
/// | `default = value` | SQL `DEFAULT` clause. A string literal becomes a SQL string, `true`/`false` become `TRUE`/`FALSE`, and a path or call such as `now()` is written as SQL. Not allowed on identity or generated columns. |
/// | `default_fn = path` | Rust function called to fill the field when an insert model is created. Cannot be combined with `default`. |
/// | `references = Table::column` | Foreign key to another table's column. |
/// | `fk_name = "..."` | Name of that foreign key constraint (default `{table}_{column}_fkey`). Needs `references`. |
/// | `on_delete = ACTION`, `on_update = ACTION` | Referential action for `references`: `CASCADE`, `SET_NULL`, `SET_DEFAULT`, `RESTRICT` or `NO_ACTION`. |
/// | `deferrable`, `initially_deferred` | Make the column's foreign key deferrable. Needs `references`. |
/// | `relation = "name"` | Name of the reverse accessor the referenced table gets through this column (see [Relations](#relations)). Needs `references`. |
/// | `many_to_many = "name"` | Name of the many-to-many accessor the referenced table gets through this link table; also makes a table with two foreign keys a link (see [Relations](#relations)). Needs `references`. |
/// | `collate = "C"` | Column collation, written quoted in the DDL. |
/// | `check = "balance >= 0"` | Column-level `CHECK` constraint. The expression is raw SQL. |
/// | `generated(stored, "expr")` or `generated(virtual, "expr")` | Generated column. Never written by inserts. `VIRTUAL` needs `PostgreSQL` 18 or later. |
///
/// The SQL type comes from the Rust type: for example `i16` is `SMALLINT`,
/// `i32` is `INTEGER`, `i64` is `BIGINT`, `f64` is `DOUBLE PRECISION`, `bool`
/// is `BOOLEAN`, `String` is `TEXT`, `Vec<u8>` is `BYTEA` and `uuid::Uuid` is
/// `UUID`. Any other type must implement `DrizzlePostgresColumn`.
///
/// # Nullability
///
/// `Option<T>` makes a column nullable. Every other field gets `NOT NULL`
/// (serial columns are implicitly `NOT NULL`).
///
/// # Comments
///
/// Doc comments become SQL comments: `///` on the struct becomes
/// `COMMENT ON TABLE`, and `///` on a field becomes `COMMENT ON COLUMN`.
/// The schema's `create_statements()` runs them right after the table.
///
/// # Generated items
///
/// For `#[PostgresTable] struct Users { ... }` the macro generates:
///
/// | Item | Purpose |
/// |---|---|
/// | `Users` | The table handle. Its fields (`users.name`) are typed columns for building queries. `Users::ddl_sql()` returns the `CREATE TABLE` statement. |
/// | `SelectUsers` | One row: every column, with `Option<T>` for nullable ones. |
/// | `PartialSelectUsers` | A row where every field is `Option<T>`, for partial selects. |
/// | `InsertUsers` | Insert builder. `InsertUsers::new(...)` takes the required columns in field order; `with_<field>(value)` sets the rest. |
/// | `UpdateUsers` | Update builder. Start from `UpdateUsers::default()` and set columns with `with_<field>(value)`. |
/// | `users` module | One type per column (`users::Name` is the type of `Users::name`), plus the alias column types. |
/// | `Users::alias::<Tag>()` | A second, independently named copy of the table for self-joins (name the alias with a type made by the prelude's `tag!` macro). |
///
/// A column is required in `InsertUsers::new` unless it is nullable, has a
/// `default` or `default_fn`, or is a serial, identity or generated column.
///
/// Row conversions are generated for every enabled driver (`postgres-sync`,
/// `tokio-postgres`, `aws-data-api`).
///
/// # Relations
///
/// With the `query` feature, `references` and table-level `foreign_key(...)`
/// also generate relation accessors for the relational query API: forward
/// and reverse ones, one-to-one and many-to-many. They are named as for
/// [`SQLiteTable`](SQLiteTable#relations).
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// use drizzle::postgres::prelude::*;
///
/// #[PostgresTable(name = "accounts")]
/// struct Accounts {
///     #[column(serial, primary)]
///     id: i32,
///     #[column(varchar(255), unique)]
///     email: String,
///     #[column(default = 0, check = "balance >= 0")]
///     balance: i64,
///     note: Option<String>,
/// }
///
/// #[PostgresTable(name = "events")]
/// struct Events {
///     #[column(identity(always), primary)]
///     id: i64,
///     #[column(references = Accounts::id, on_delete = CASCADE)]
///     account_id: i32,
/// }
///
/// assert_eq!(
///     Accounts::ddl_sql(),
///     "CREATE TABLE \"accounts\" (\n\
///     \t\"id\" SERIAL,\n\
///     \t\"email\" VARCHAR(255) NOT NULL,\n\
///     \t\"balance\" BIGINT DEFAULT 0 NOT NULL,\n\
///     \t\"note\" TEXT,\n\
///     \tPRIMARY KEY(\"id\"),\n\
///     \tCONSTRAINT \"accounts_email_key\" UNIQUE(\"email\"),\n\
///     \tCONSTRAINT \"accounts_balance_check\" CHECK (balance >= 0)\n\
///     );"
/// );
/// assert_eq!(
///     Events::ddl_sql(),
///     "CREATE TABLE \"events\" (\n\
///     \t\"id\" BIGINT GENERATED ALWAYS AS IDENTITY NOT NULL,\n\
///     \t\"account_id\" INTEGER NOT NULL,\n\
///     \tPRIMARY KEY(\"id\"),\n\
///     \tCONSTRAINT \"events_account_id_fkey\" FOREIGN KEY (\"account_id\") REFERENCES \"accounts\"(\"id\") ON DELETE CASCADE\n\
///     );"
/// );
///
/// // `id` is serial and `balance` has a default, so only `email` is required.
/// let account = InsertAccounts::new("ada@example.com").with_note("first");
/// # let _ = account;
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
///
/// JSON columns:
///
/// ```rust
/// # #[cfg(all(feature = "postgres", feature = "serde"))]
/// # fn main() {
/// use drizzle::postgres::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// struct Preferences {
///     theme: String,
/// }
///
/// #[PostgresTable]
/// struct Settings {
///     #[column(serial, primary)]
///     id: i32,
///     #[column(jsonb)]
///     preferences: Preferences,
///     #[column(json)]
///     raw: Option<serde_json::Value>,
/// }
/// # }
/// # #[cfg(not(all(feature = "postgres", feature = "serde")))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PostgresTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::postgres::table::TableAttributes);

    match crate::postgres::table::table_attr_macro(&input, &attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `PostgreSQL` view or materialized view from a struct.
///
/// Each named field is a column of the view. The view is queried like a
/// table: the macro generates the same column handles, `Select*` models and
/// row conversions as [`PostgresTable`]. Field attributes are the
/// `#[column(...)]` attributes of [`PostgresTable`]; usually none are needed.
///
/// # Attributes
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "active_users"` | SQL view name. Defaults to the struct name in `snake_case`. |
/// | `schema = "app"` | Create the view in this schema. Defaults to `public`. |
/// | `definition = "SELECT ..."` | The view's query as raw SQL. |
/// | `definition = expr` | The view's query as a query-builder expression, rendered when the view is created. |
/// | `query(...)` | The view's query in the typed DSL described on [`SQLiteView`](SQLiteView#the-query-dsl), rendered at compile time. |
/// | `materialized` | Create a materialized view. |
/// | `with_no_data` | Materialized view created `WITH NO DATA`. |
/// | `using = "heap"` | Table access method of a materialized view. |
/// | `tablespace = "name"` | Tablespace of a materialized view. |
/// | `with = ViewWithOptionDef::new()...` | `WITH (...)` storage options, such as `security_barrier()`. |
/// | `existing` | The view already exists in the database: drizzle queries it but never creates it. |
///
/// A view needs one of `definition` or `query(...)` unless it is `existing`.
///
/// # Generated items
///
/// Besides the table-like items, the view gets `VIEW_NAME`,
/// `VIEW_DEFINITION_SQL` and `ddl_sql()`, which returns the `CREATE VIEW`
/// statement.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// use drizzle::postgres::prelude::*;
///
/// #[PostgresTable(name = "accounts")]
/// struct Accounts {
///     #[column(serial, primary)]
///     id: i32,
///     balance: i64,
/// }
///
/// #[PostgresView(definition = "SELECT id FROM accounts WHERE balance > 0")]
/// struct FundedAccounts {
///     id: i32,
/// }
///
/// #[PostgresView(materialized, with_no_data, query(select(Accounts::id), from(Accounts)))]
/// struct AccountIds {
///     id: i32,
/// }
///
/// assert_eq!(
///     FundedAccounts::ddl_sql(),
///     r#"CREATE VIEW "funded_accounts" AS SELECT id FROM accounts WHERE balance > 0"#
/// );
/// assert_eq!(
///     AccountIds::ddl_sql(),
///     r#"CREATE MATERIALIZED VIEW "account_ids" AS SELECT "public"."accounts"."id" AS "id" FROM "public"."accounts" WITH NO DATA"#
/// );
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PostgresView(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::postgres::view::ViewAttributes);

    match crate::postgres::view::view_attr_macro(&input, &attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `PostgreSQL` index on one or more columns of a table.
///
/// Apply it to a tuple struct whose fields name the indexed columns as
/// `Table::column`. All columns must belong to the same table. Add the index
/// to a [`PostgresSchema`] so `db.create()` and migrations include it.
///
/// # Attributes
///
/// | Attribute | Meaning |
/// |---|---|
/// | `unique` | Create a `UNIQUE` index. |
/// | `name = "..."` | SQL index name. Defaults to the struct name in `snake_case`, with `_idx` appended unless it already ends in `_idx` or `_index`. |
/// | `concurrent` | `CREATE INDEX CONCURRENTLY`, which does not block writes. |
/// | `method = "gin"` | Index method: `btree`, `hash`, `gin`, `gist`, `spgist` or `brin`. |
/// | `tablespace = "name"` | Accepted, but not yet written into the `CREATE INDEX` statement. |
/// | `where = "..."` | Partial index predicate, as raw SQL. Column names inside the string are not checked. |
///
/// # Generated items
///
/// The struct becomes a unit struct with `new()`, and `ddl_sql()` returns its
/// `CREATE INDEX` statement. A `unique` index can also be passed to an
/// insert's `on_conflict(...)` as the conflict target.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// use drizzle::postgres::prelude::*;
///
/// #[PostgresTable(name = "users")]
/// struct Users {
///     #[column(serial, primary)]
///     id: i32,
///     email: String,
///     deleted_at: Option<i64>,
/// }
///
/// #[PostgresIndex(unique, method = "btree")]
/// struct UsersEmail(Users::email);
///
/// #[PostgresIndex(concurrent, where = "deleted_at IS NULL")]
/// struct LiveUsersIdx(Users::email, Users::deleted_at);
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     users: Users,
///     users_email: UsersEmail,
///     live_users_idx: LiveUsersIdx,
/// }
///
/// assert_eq!(
///     UsersEmail::ddl_sql(),
///     r#"CREATE UNIQUE INDEX "users_email_idx" ON "public"."users" USING btree("email")"#
/// );
/// assert_eq!(
///     LiveUsersIdx::ddl_sql(),
///     r#"CREATE INDEX CONCURRENTLY "live_users_idx" ON "public"."users"("email", "deleted_at") WHERE deleted_at IS NULL"#
/// );
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PostgresIndex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_input = syn::parse_macro_input!(attr as crate::postgres::index::IndexAttributes);

    match crate::postgres::index::postgres_index_attr_macro(&attr_input, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `PostgreSQL` row-level security policy on a table.
///
/// Apply it to a tuple struct holding exactly one table type. Policies apply
/// only once row-level security is on, so mark the table `rls`, and add the
/// policy to a [`PostgresSchema`] so it is created with the table. See
/// [CREATE POLICY](https://www.postgresql.org/docs/current/sql-createpolicy.html).
///
/// # Attributes
///
/// Keys are case-insensitive; all values are string literals.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "..."` | Policy name. Defaults to the struct name in `snake_case`. |
/// | `AS = "PERMISSIVE"` | `PERMISSIVE` (the default) or `RESTRICTIVE`. |
/// | `FOR = "SELECT"` | Command it applies to: `ALL`, `SELECT`, `INSERT`, `UPDATE` or `DELETE`. |
/// | `TO("role", ...)` or `TO = "role"` | Roles it applies to. Bare identifiers work too: `TO(public)`. |
/// | `USING = "..."` | Raw SQL condition for rows that may be read or changed. |
/// | `WITH_CHECK = "..."` | Raw SQL condition new rows must satisfy. |
///
/// # Generated items
///
/// The struct becomes a unit struct with `new()`, `TO_ROLES`, and `ddl_sql()`,
/// which returns the `CREATE POLICY` statement.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// use drizzle::postgres::prelude::*;
///
/// #[PostgresTable(name = "documents", rls)]
/// struct Documents {
///     #[column(serial, primary)]
///     id: i32,
///     owner: String,
/// }
///
/// #[PostgresPolicy(FOR = "SELECT", TO("public"), USING = "owner = current_user")]
/// struct OwnerCanRead(Documents);
///
/// assert_eq!(
///     OwnerCanRead::ddl_sql(),
///     r#"CREATE POLICY "owner_can_read" ON "public"."documents" AS PERMISSIVE FOR SELECT TO PUBLIC USING (owner = current_user);"#
/// );
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
#[cfg(feature = "postgres")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PostgresPolicy(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_input = syn::parse_macro_input!(attr as crate::postgres::policy::PolicyAttributes);

    match crate::postgres::policy::postgres_policy_attr_macro(&attr_input, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive a `MySQL` inline `ENUM` column type for a fieldless enum.
///
/// The column stores the variant names, as written and in declaration order:
/// `ENUM('Draft', 'Published')`. Mark the column with `#[column(ENUM)]` in a
/// [`MySQLTable`]. Unlike `PostgreSQL`, there is no separate type to create,
/// so the enum does not go in the [`MySQLSchema`].
///
/// The derive takes no attributes. The enum needs at least one variant; every
/// variant must be a unit variant without an explicit discriminant; `#[repr]`
/// is rejected; and variant names may not contain `'` or `\`.
///
/// # Generated impls
///
/// - The `MySQLEnum` trait (`SQL_TYPE`, `VARIANTS`, `variant_name()`).
/// - `Display`, `FromStr`, `AsRef<str>`, and `TryFrom<&str>` / `TryFrom<String>`,
///   all using the variant name.
/// - The `MySQL` value, expression and row conversions, so the enum works as a
///   column type, a bound parameter, and a selected value.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mysql")]
/// # fn main() {
/// use drizzle::mysql::prelude::*;
///
/// #[derive(MySQLEnum, Clone, Copy, Debug, PartialEq)]
/// enum Status {
///     Draft,
///     Published,
/// }
///
/// #[MySQLTable]
/// struct Posts {
///     #[column(primary, auto_increment)]
///     id: u64,
///     #[column(ENUM)]
///     status: Status,
/// }
///
/// assert_eq!(Status::Published.to_string(), "Published");
/// assert_eq!("Draft".parse::<Status>().unwrap(), Status::Draft);
/// assert!(Posts::create_table_sql().contains("`status` ENUM('Draft', 'Published') NOT NULL"));
/// # }
/// # #[cfg(not(feature = "mysql"))]
/// # fn main() {}
/// ```
#[cfg(feature = "mysql")]
#[proc_macro_derive(MySQLEnum)]
pub fn mysql_enum_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match &input.data {
        syn::Data::Enum(data) => {
            match crate::mysql::r#enum::generate_enum_impl(&input.ident, data, &input.attrs) {
                Ok(tokens) => tokens.into(),
                Err(err) => err.to_compile_error().into(),
            }
        }
        _ => syn::Error::new_spanned(input, "#[derive(MySQLEnum)] can only be applied to enums")
            .to_compile_error()
            .into(),
    }
}

/// Define a `MySQL` table from a struct.
///
/// Each named field becomes a column. The macro replaces the struct with a
/// zero-sized table handle whose fields are column handles, and generates the
/// models used to insert, select and update rows. See
/// [CREATE TABLE](https://dev.mysql.com/doc/refman/8.4/en/create-table.html)
/// for the SQL side.
///
/// # Table attributes
///
/// Written as `#[MySQLTable(...)]`. All are optional.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "users"` | SQL table name. Defaults to the struct name in `snake_case`. |
/// | `database = "app"` (or `schema`) | Qualify the table with a database: `` `app`.`users` ``. |
/// | `temporary` | `CREATE TEMPORARY TABLE`. |
/// | `engine = "InnoDB"` | `ENGINE=` table option. |
/// | `charset = "utf8mb4"` (or `default_charset`) | `DEFAULT CHARACTER SET=` table option. |
/// | `collate = "utf8mb4_0900_ai_ci"` | `COLLATE=` table option. |
/// | `comment = "..."` | Table `COMMENT`. Without it, the struct's doc comment is used. |
/// | `unique(a, b)` or `unique(columns(a, b), name = "...")` | Table-level `UNIQUE` constraint over the named fields. |
/// | `check(expr = "a < b", name = "...")` | Table-level `CHECK` constraint. The expression is raw SQL. |
/// | `foreign_key(columns(a, b), references(Parent, x, y), name = "...", on_delete = "CASCADE", on_update = "...", relation = "...", many_to_many = "...")` | Composite foreign key from fields `a, b` to `Parent`'s fields `x, y`. `name` defaults to `{table}_{a}_fkey`. `relation` and `many_to_many` name its relations (see [Relations](#relations)). |
///
/// `engine`, `charset` and `collate` take ASCII letters, digits and `_` only.
///
/// # Column attributes
///
/// Written as `#[column(...)]` on a field. All are optional.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `primary` (or `primary_key`) | Primary key. Put it on several fields for a composite key. The field cannot be an `Option`. |
/// | `unique` | `UNIQUE` constraint. |
/// | `auto_increment` | `AUTO_INCREMENT`. Only on an integer column that is `primary` or `unique`, at most one per table, and not with `default` or `generated`. |
/// | `serial` | `BIGINT UNSIGNED NOT NULL AUTO_INCREMENT UNIQUE`, on a `u64` field. |
/// | `ENUM` (uppercase only) | The field's type derives [`MySQLEnum`]; the column is an inline `ENUM(...)`. |
/// | `set("a", "b")` | A `SET('a', 'b')` column, with 1 to 64 values. |
/// | `json` | A `JSON` column. A `serde` payload type needs the `serde` feature. |
/// | SQL type keys | Override the SQL type inferred from the Rust type: `varchar(n)`, `char(n)`, `text`, `longtext`, `binary(n)`, `varbinary(n)`, `blob`, `tinyint`, `int`, `bigint`, `decimal(p, s)`, `double`, `date`, `datetime`, `timestamp`, `year` and more, with `_unsigned` variants for the numeric types (`int_unsigned`). The signedness must match the Rust type. |
/// | `name = "col"` | SQL column name. Defaults to the field name in `snake_case`. |
/// | `default = value` | SQL `DEFAULT` clause. A literal is written as SQL (`true` becomes `TRUE`), in parentheses on `TEXT`, `BLOB` and `JSON` columns. A path or call is written in parentheses, except `CURRENT_TIMESTAMP` on `DATETIME`/`TIMESTAMP` columns. |
/// | `default_fn = path` | Rust function called to fill the field when an insert model is created. Cannot be combined with `default`. |
/// | `on_update = "CURRENT_TIMESTAMP"` | Column `ON UPDATE` clause, given as a string. Only on `DATETIME` and `TIMESTAMP` columns. |
/// | `references = Table::column` | Foreign key to another table's column. |
/// | `on_delete = ACTION`, `on_update = ACTION` | Referential action for `references`, as a bare identifier: `CASCADE`, `SET_NULL`, `RESTRICT` or `NO_ACTION`. InnoDB rejects `SET_DEFAULT`. |
/// | `relation = "name"` | Name of the reverse accessor the referenced table gets through this column (see [Relations](#relations)). Needs `references`. |
/// | `many_to_many = "name"` | Name of the many-to-many accessor the referenced table gets through this link table; also makes a table with two foreign keys a link (see [Relations](#relations)). Needs `references`. |
/// | `charset = "..."` (or `character_set`), `collate = "..."` | Column character set and collation, on character, text, `ENUM` and `SET` columns. |
/// | `comment = "..."` | Column `COMMENT`. |
/// | `check = "score >= 0"` | Column-level `CHECK` constraint. The expression is raw SQL. |
/// | `generated(stored, "expr")` or `generated(virtual, "expr")` | Generated column. Never written by inserts. Cannot be combined with `default`, `default_fn` or `on_update`. |
///
/// Without a type key, the SQL type comes from the Rust type: `i8`/`u8` are
/// `TINYINT`/`TINYINT UNSIGNED`, `i16` `SMALLINT`, `i32` `INT`, `i64`
/// `BIGINT` (each with an `UNSIGNED` form for the unsigned type), `f32` is
/// `FLOAT`, `f64` `DOUBLE`, `bool` `BOOLEAN`, `String` `TEXT`, `Vec<u8>`
/// `BLOB` and `uuid::Uuid` `BINARY(16)`. Any other type must implement
/// `DrizzleMySQLColumn`. `TEXT`, `BLOB` and `JSON` columns cannot be indexed
/// directly; give a key column a bounded type such as `varchar(255)`.
///
/// # Nullability
///
/// `Option<T>` makes a column nullable. Every other field gets `NOT NULL`.
///
/// # Generated items
///
/// For `#[MySQLTable] struct Users { ... }` the macro generates the table
/// handle `Users` (with `TABLE_NAME` and `ddl_sql()`, which returns the
/// `CREATE TABLE` statement), the models `SelectUsers`, `PartialSelectUsers`,
/// `InsertUsers` and `UpdateUsers`, a `users` module with one type per column
/// (`users::Name` is the type of `Users::name`), and `Users::alias::<Tag>()`
/// for self-joins. A column is required in `InsertUsers::new` unless it is
/// nullable or has a `default`, `default_fn`, `auto_increment` or
/// `generated` value. Row conversions are driver-neutral.
///
/// # Relations
///
/// `references = Table::column` declares a foreign key. With the `query`
/// feature it and table-level `FOREIGN_KEY(...)` also generate relation
/// accessors: forward and reverse ones, one-to-one and many-to-many. They are
/// named as for [`SQLiteTable`](SQLiteTable#relations).
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mysql")]
/// # fn main() {
/// use drizzle::mysql::prelude::*;
///
/// #[MySQLTable(name = "accounts", engine = "InnoDB")]
/// struct Accounts {
///     #[column(primary, auto_increment)]
///     id: u64,
///     #[column(varchar(255), unique)]
///     email: String,
///     #[column(default = 0)]
///     login_count: u32,
///     nickname: Option<String>,
/// }
///
/// let sql = Accounts::ddl_sql();
/// assert!(sql.starts_with("CREATE TABLE `accounts` ("));
/// assert!(sql.contains("`id` BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT"));
/// assert!(sql.contains("`login_count` INT UNSIGNED NOT NULL DEFAULT 0"));
/// assert!(sql.ends_with(") ENGINE=InnoDB;"));
/// # }
/// # #[cfg(not(feature = "mysql"))]
/// # fn main() {}
/// ```
#[cfg(feature = "mysql")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn MySQLTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let attrs = parse_macro_input!(attr as crate::mysql::table::TableAttributes);

    match crate::mysql::table::table_attr_macro(&input, &attrs) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `MySQL` view from a struct.
///
/// Each named field is a column of the view. The view is queried like a
/// table: the macro generates the same column handles, `Select*` models and
/// row conversions as [`MySQLTable`]. Field attributes are the
/// `#[column(...)]` attributes of [`MySQLTable`].
///
/// # Attributes
///
/// | Attribute | Meaning |
/// |---|---|
/// | `name = "active_users"` | SQL view name. Defaults to the struct name in `snake_case`. |
/// | `database = "app"` (or `schema`) | Qualify the view with a database. |
/// | `definition = "SELECT ..."` | The view's query as raw SQL. |
/// | `definition = expr` | The view's query as a query-builder expression, rendered when the view is created. |
/// | `query(...)` | The view's query in the typed DSL described on [`SQLiteView`](SQLiteView#the-query-dsl), rendered at compile time. `full_join` is not available. |
/// | `algorithm = "merge"` | `ALGORITHM=`: `undefined`, `merge` or `temptable`. |
/// | `sql_security = "invoker"` (or `security`) | `SQL SECURITY`: `definer` or `invoker`. |
/// | `check_option = "local"` | `WITH ... CHECK OPTION`: `cascaded` or `local`. A bare `check_option` or `with_check_option` means `cascaded`. |
/// | `existing` | The view already exists in the database: drizzle queries it but never creates it. |
///
/// A view needs exactly one of `definition` or `query(...)` unless it is
/// `existing`, which allows neither.
///
/// # Generated items
///
/// Besides the table-like items, the view gets `VIEW_NAME`,
/// `VIEW_DEFINITION_SQL` and `ddl_sql()`, which returns the `CREATE VIEW`
/// statement.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mysql")]
/// # fn main() {
/// use drizzle::mysql::prelude::*;
///
/// #[MySQLView(
///     name = "account_emails",
///     definition = "SELECT id, email FROM accounts",
///     algorithm = "merge",
///     sql_security = "invoker",
///     check_option = "local"
/// )]
/// struct AccountEmails {
///     id: u64,
///     #[column(varchar(255))]
///     email: String,
/// }
///
/// assert_eq!(
///     AccountEmails::ddl_sql(),
///     "CREATE ALGORITHM=MERGE SQL SECURITY INVOKER VIEW `account_emails` AS SELECT id, email FROM accounts WITH LOCAL CHECK OPTION;"
/// );
/// # }
/// # #[cfg(not(feature = "mysql"))]
/// # fn main() {}
/// ```
#[cfg(feature = "mysql")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn MySQLView(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let attrs = parse_macro_input!(attr as crate::mysql::view::ViewAttributes);

    match crate::mysql::view::view_attr_macro(&input, &attrs) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a `MySQL` index on one or more columns of a table.
///
/// Apply it to a tuple struct whose fields name the indexed columns as
/// `Table::column`. All columns must belong to the same table. Add the index
/// to a [`MySQLSchema`] so `db.create()` and migrations include it.
///
/// # Attributes
///
/// Keys are case-insensitive.
///
/// | Attribute | Meaning |
/// |---|---|
/// | `unique` | Create a `UNIQUE` index. |
/// | `name = "..."` | SQL index name. Defaults to the struct name in `snake_case`. |
/// | `using = "btree"` | Index method: `btree` or `hash`. |
/// | `algorithm = "inplace"` | `ALGORITHM=`: `default`, `inplace` or `copy`. |
/// | `lock = "none"` | `LOCK=`: `default`, `none`, `shared` or `exclusive`. |
///
/// `MySQL` has no partial indexes, so `where = ...` is rejected.
///
/// Each field may carry `#[index(...)]` to shape its key part: `asc` or
/// `desc`, `prefix = n` to index the first `n` characters, or
/// `expr = "lower(email)"` to index an expression instead of the column
/// (the field's column then only names the table). `expr` and `prefix`
/// cannot be combined.
///
/// # Generated items
///
/// The struct becomes a unit struct with `new()`, and `DDL_SQL` /
/// `ddl_sql()` hold its `CREATE INDEX` statement.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mysql")]
/// # fn main() {
/// use drizzle::mysql::prelude::*;
///
/// #[MySQLTable(name = "accounts")]
/// struct Accounts {
///     #[column(primary, auto_increment)]
///     id: u64,
///     #[column(varchar(255))]
///     email: String,
/// }
///
/// #[MySQLIndex(unique)]
/// struct AccountsEmailIdx(Accounts::email);
///
/// #[MySQLIndex(using = "btree", algorithm = "inplace", lock = "none")]
/// struct AccountsSearchIdx(
///     #[index(prefix = 24, desc)] Accounts::email,
///     #[index(expr = "lower(email)", asc)] Accounts::id,
/// );
///
/// assert_eq!(
///     AccountsEmailIdx::ddl_sql(),
///     "CREATE UNIQUE INDEX `accounts_email_idx` ON `accounts`(`email`);"
/// );
/// assert_eq!(
///     AccountsSearchIdx::ddl_sql(),
///     "CREATE INDEX `accounts_search_idx` USING BTREE ON `accounts`(`email`(24) DESC, (lower(email)) ASC) ALGORITHM=INPLACE LOCK=NONE;"
/// );
/// # }
/// # #[cfg(not(feature = "mysql"))]
/// # fn main() {}
/// ```
#[cfg(feature = "mysql")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn MySQLIndex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let attrs = parse_macro_input!(attr as crate::mysql::index::IndexAttributes);

    match crate::mysql::index::mysql_index_attr_macro(attrs, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive a `MySQL` schema: the set of tables, indexes and views an
/// application uses.
///
/// Apply it to a struct with named fields, one per table ([`MySQLTable`]),
/// index ([`MySQLIndex`]) or view ([`MySQLView`]). A driver's
/// `Drizzle::new(conn)` returns the schema next to the connection, and the
/// schema is what `db.create()` and migrations work from. [`MySQLEnum`]
/// types are inline column types and are not listed here.
///
/// The derive takes no attributes.
///
/// # Generated items
///
/// - `Schema::new()` (a `const fn`) and `Default`, building every handle.
/// - `Clone`, `Copy` and `Debug`. Do not derive these (or `Default`) yourself:
///   the macro reports an error if they appear in a `#[derive]` on the struct.
/// - `schema.items()` and `From<Schema>` for the tuple of fields.
/// - `SQLSchemaImpl`, whose `create_statements()` returns the `CREATE`
///   statements: tables ordered so referenced tables come first, each table's
///   indexes right after it, then views. Tables in a reference cycle are
///   created with the session's foreign-key checks off, the way `mysqldump`
///   does. An index whose table is not in the schema is an error there.
/// - The migrations `Schema` trait, so the CLI and `migrate()` can snapshot it.
///
/// As with [`SQLiteSchema`], every table a foreign key points to must be in
/// the schema, or the derive fails to compile.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mysql")]
/// # fn main() -> drizzle::Result<()> {
/// use drizzle::core::SQLSchemaImpl;
/// use drizzle::mysql::prelude::*;
///
/// #[MySQLTable(name = "users")]
/// struct Users {
///     #[column(primary, auto_increment)]
///     id: u64,
///     #[column(varchar(255))]
///     email: String,
/// }
///
/// #[MySQLTable(name = "posts")]
/// struct Posts {
///     #[column(primary, auto_increment)]
///     id: u64,
///     #[column(references = Users::id)]
///     author_id: u64,
/// }
///
/// #[MySQLIndex(unique)]
/// struct UsersEmailIdx(Users::email);
///
/// #[derive(MySQLSchema)]
/// struct Schema {
///     posts: Posts,
///     users: Users,
///     users_email_idx: UsersEmailIdx,
/// }
///
/// let statements: Vec<String> = Schema::new().create_statements()?.collect();
/// assert!(statements[0].starts_with("CREATE TABLE `users`"));
/// assert_eq!(statements[1], "CREATE UNIQUE INDEX `users_email_idx` ON `users`(`email`);");
/// assert!(statements[2].starts_with("CREATE TABLE `posts`"));
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "mysql"))]
/// # fn main() {}
/// ```
#[cfg(feature = "mysql")]
#[proc_macro_derive(MySQLSchema)]
pub fn mysql_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::mysql::generate_mysql_schema_derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive row decoding for a custom result struct on `MySQL`.
///
/// The `MySQL` counterpart of [`SQLiteFromRow`], with the same attributes and
/// the same `Struct::Select` selector:
///
/// | Attribute | Where | Meaning |
/// |---|---|---|
/// | `#[from(Table)]` | struct | Default table for fields without `#[column(...)]`. |
/// | `#[column(Table::field)]` | field | Read this field from `Table::field`. |
///
/// The row conversions are driver-neutral: the struct gets `TryFrom<&Row>`
/// and the row-decoding traits for drizzle's `MySQL` row wrapper, which both
/// `mysql` drivers use. Fields always decode by position (field 0 is column
/// 0), using each field type's own row conversion.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mysql")]
/// # fn main() {
/// use drizzle::mysql::prelude::*;
///
/// #[MySQLTable(name = "accounts")]
/// struct Accounts {
///     #[column(primary, auto_increment)]
///     id: u64,
///     #[column(varchar(255))]
///     email: String,
/// }
///
/// #[derive(MySQLFromRow)]
/// #[from(Accounts)]
/// struct AccountRow {
///     id: u64,
///     #[column(Accounts::email)]
///     address: String,
/// }
///
/// // Pass to `select(...)`: it selects `accounts.id` and `accounts.email`,
/// // aliased to the field names `id` and `address`.
/// let selector = AccountRow::Select;
/// # let _ = selector;
/// # }
/// # #[cfg(not(feature = "mysql"))]
/// # fn main() {}
/// ```
#[cfg(feature = "mysql")]
#[proc_macro_derive(MySQLFromRow, attributes(column, from))]
pub fn mysql_from_row_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::fromrow::generate_mysql_from_row_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
