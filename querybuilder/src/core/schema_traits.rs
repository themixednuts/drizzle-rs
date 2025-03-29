/// Marker trait indicating that a table `T` is part of a schema represented by the marker type `S`.
///
/// This trait is used as a bound on methods like `QueryBuilder::from` and `QueryBuilder::join`
/// to ensure that only tables declared within a specific `schema!` macro invocation can be used
/// with the resulting query builder.
///
/// ## Compile-Time Errors
///
/// If you encounter a compile-time error message like:
/// ```text
/// the trait bound `YourTable: querybuilder::core::schema_traits::IsInSchema<...some_marker_module::SchemaMarker>` is not satisfied
/// ```
/// **This almost always means that `YourTable` was not included in the list of tables when the corresponding `schema!` macro was called.**
///
/// For example:
/// ```compile_fail
/// # use querybuilder::prelude::*;
/// # use querybuilder::sqlite::query_builder::{QueryBuilder, SQLiteQueryBuilder};
/// # use procmacros::SQLiteTable;
/// # #[SQLiteTable(name = "users")] struct User { id: i32 };
/// # #[SQLiteTable(name = "posts")] struct Post { id: i32 };
/// // Schema only includes User
/// let qb = querybuilder::schema!([User]);
///
/// // This line will FAIL because Post is not in the schema:
/// let posts = qb.from::<Post>().select_all(); // <-- Error happens here
/// ```
///
/// To fix this, ensure that `YourTable` is listed within the `[...]` of the `schema!` macro call
/// that created the query builder instance you are trying to use.
///
/// Implementations of this trait are generated automatically by the `schema!` macro for each table
/// listed within it, associating those tables with the unique, generated schema marker type `S`.
pub trait IsInSchema<S> {}
