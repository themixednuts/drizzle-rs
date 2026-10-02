//! `REFRESH MATERIALIZED VIEW` statements for `PostgreSQL`.
//!
//! Build the statement with [`refresh_materialized_view`] or
//! [`RefreshMaterializedView::new`] and run it with a driver's
//! `execute(...)`, which accepts any `ToSQL` value.
//!
//! # Examples
//!
//! ```rust
//! use drizzle_postgres::builder::refresh::RefreshMaterializedView;
//! # use drizzle_core::traits::{SQLTableInfo, SQLViewInfo};
//! # struct UserStats;
//! # impl SQLTableInfo for UserStats {
//! #     fn name(&self) -> &'static str { "user_stats" }
//! #     fn schema(&self) -> Option<&'static str> { None }
//! # }
//! # impl SQLViewInfo for UserStats {
//! #     fn definition_sql(&self) -> std::borrow::Cow<'static, str> { "SELECT 1".into() }
//! #     fn is_materialized(&self) -> bool { true }
//! # }
//! # let user_stats = UserStats;
//! use drizzle_core::ToSQL;
//!
//! // `user_stats` stands for a materialized view defined with `#[PostgresView]`.
//! let refresh = RefreshMaterializedView::new(&user_stats);
//! assert_eq!(refresh.to_sql().sql(), r#"REFRESH MATERIALIZED VIEW "user_stats""#);
//!
//! // Keep the view readable while it refreshes (needs a unique index).
//! let refresh = RefreshMaterializedView::new(&user_stats).concurrently();
//! assert_eq!(
//!     refresh.to_sql().sql(),
//!     r#"REFRESH MATERIALIZED VIEW CONCURRENTLY "user_stats""#
//! );
//!
//! // Empty the view; it cannot be queried until refreshed again.
//! let refresh = RefreshMaterializedView::new(&user_stats).with_no_data();
//! assert_eq!(
//!     refresh.to_sql().sql(),
//!     r#"REFRESH MATERIALIZED VIEW "user_stats" WITH NO DATA"#
//! );
//! ```

use crate::values::PostgresValue;
use core::marker::PhantomData;
use drizzle_core::traits::{SQLTableInfo, SQLViewInfo};
use drizzle_core::{SQL, ToSQL, Token};

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// [`RefreshMaterializedView`] state before any option is chosen.
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshInitial;

/// [`RefreshMaterializedView`] state after `.concurrently()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshConcurrently;

/// [`RefreshMaterializedView`] state after `.with_no_data()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshWithNoData;

//------------------------------------------------------------------------------
// RefreshMaterializedView Builder
//------------------------------------------------------------------------------

/// Builds a `REFRESH MATERIALIZED VIEW` statement.
///
/// ```sql
/// REFRESH MATERIALIZED VIEW [ CONCURRENTLY ] view_name [ WITH [ NO ] DATA ]
/// ```
///
/// `CONCURRENTLY` and `WITH NO DATA` cannot be combined; the state parameter
/// enforces this. A view outside the `public` schema is schema-qualified.
/// See the [module docs](self) for examples.
#[derive(Debug, Clone)]
pub struct RefreshMaterializedView<'a, State = RefreshInitial> {
    sql: SQL<'a, PostgresValue<'a>>,
    _state: PhantomData<State>,
}

impl<'a> RefreshMaterializedView<'a, RefreshInitial> {
    /// Starts `REFRESH MATERIALIZED VIEW view`.
    #[must_use]
    pub fn new<V: SQLViewInfo>(view: &'a V) -> Self {
        Self {
            sql: SQL::from_iter([Token::REFRESH, Token::MATERIALIZED, Token::VIEW])
                .append(qualified_view_name(view)),
            _state: PhantomData,
        }
    }

    /// Adds `CONCURRENTLY`: the view stays readable during the refresh.
    ///
    /// `PostgreSQL` requires a unique index on the materialized view for this.
    /// Cannot be combined with `WITH NO DATA`.
    #[must_use]
    pub fn concurrently(self) -> RefreshMaterializedView<'a, RefreshConcurrently> {
        // Rebuild as REFRESH MATERIALIZED VIEW CONCURRENTLY <name>: the name
        // chunks are everything after the three leading keywords.
        let mut sql = SQL::from_iter([
            Token::REFRESH,
            Token::MATERIALIZED,
            Token::VIEW,
            Token::CONCURRENTLY,
        ]);
        for chunk in self.sql.chunks.into_iter().skip(3) {
            sql = sql.push(chunk);
        }

        RefreshMaterializedView {
            sql,
            _state: PhantomData,
        }
    }

    /// Adds `WITH NO DATA`: the view is emptied instead of refreshed.
    ///
    /// The view cannot be queried until a later refresh fills it. Cannot be
    /// combined with `CONCURRENTLY`.
    #[must_use]
    pub fn with_no_data(self) -> RefreshMaterializedView<'a, RefreshWithNoData> {
        RefreshMaterializedView {
            sql: self.sql.push(Token::WITH).push(Token::NO).push(Token::DATA),
            _state: PhantomData,
        }
    }

    /// Adds `WITH DATA`, the default behaviour, explicitly.
    #[must_use]
    pub fn with_data(self) -> Self {
        Self {
            sql: self.sql.push(Token::WITH).push(Token::DATA),
            _state: PhantomData,
        }
    }
}

/// `"schema"."name"` for views in a non-default schema, otherwise the bare name
/// so the statement resolves through `search_path` exactly like the view's own
/// DDL (which also leaves `public` unqualified).
fn qualified_view_name<'a, V: SQLViewInfo>(view: &V) -> SQL<'a, PostgresValue<'a>> {
    let name = SQL::ident(view.name());
    match SQLTableInfo::schema(view) {
        Some(schema) if schema != "public" => SQL::ident(schema).push(Token::DOT).append(name),
        _ => name,
    }
}

//------------------------------------------------------------------------------
// ToSQL implementations
//------------------------------------------------------------------------------

impl<'a, State> ToSQL<'a, PostgresValue<'a>> for RefreshMaterializedView<'a, State> {
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
        self.sql.clone()
    }
}

//------------------------------------------------------------------------------
// Helper function for the query builder
//------------------------------------------------------------------------------

/// Starts `REFRESH MATERIALIZED VIEW view`. Same as [`RefreshMaterializedView::new`].
///
/// # Examples
///
/// ```rust
/// use drizzle_postgres::builder::refresh_materialized_view;
/// # use drizzle_core::traits::{SQLTableInfo, SQLViewInfo};
/// # struct UserStats;
/// # impl SQLTableInfo for UserStats {
/// #     fn name(&self) -> &'static str { "user_stats" }
/// #     fn schema(&self) -> Option<&'static str> { None }
/// # }
/// # impl SQLViewInfo for UserStats {
/// #     fn definition_sql(&self) -> std::borrow::Cow<'static, str> { "SELECT 1".into() }
/// #     fn is_materialized(&self) -> bool { true }
/// # }
/// # let user_stats = UserStats;
/// use drizzle_core::ToSQL;
///
/// let refresh = refresh_materialized_view(&user_stats).concurrently();
/// assert_eq!(
///     refresh.to_sql().sql(),
///     r#"REFRESH MATERIALIZED VIEW CONCURRENTLY "user_stats""#
/// );
/// ```
pub fn refresh_materialized_view<V: SQLViewInfo>(
    view: &V,
) -> RefreshMaterializedView<'_, RefreshInitial> {
    RefreshMaterializedView::new(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock view for testing
    struct TestView;

    impl drizzle_core::traits::SQLTableInfo for TestView {
        fn name(&self) -> &'static str {
            "user_stats"
        }

        fn schema(&self) -> Option<&'static str> {
            Some("public")
        }
    }

    impl SQLViewInfo for TestView {
        fn definition_sql(&self) -> std::borrow::Cow<'static, str> {
            "SELECT * FROM users".into()
        }

        fn is_materialized(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_basic_refresh() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view);
        let sql = refresh.to_sql();

        assert_eq!(sql.sql(), r#"REFRESH MATERIALIZED VIEW "user_stats""#);
    }

    #[test]
    fn test_concurrent_refresh() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view).concurrently();
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW CONCURRENTLY "user_stats""#
        );
    }

    #[test]
    fn test_refresh_with_no_data() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view).with_no_data();
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW "user_stats" WITH NO DATA"#
        );
    }

    #[test]
    fn test_refresh_with_data() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view).with_data();
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW "user_stats" WITH DATA"#
        );
    }

    struct ExplicitSchemaView;

    impl drizzle_core::traits::SQLTableInfo for ExplicitSchemaView {
        fn name(&self) -> &'static str {
            "user_stats"
        }

        fn schema(&self) -> Option<&'static str> {
            Some("analytics")
        }
    }

    impl SQLViewInfo for ExplicitSchemaView {
        fn definition_sql(&self) -> std::borrow::Cow<'static, str> {
            "SELECT * FROM users".into()
        }

        fn is_materialized(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_non_public_schema_is_qualified() {
        let view = ExplicitSchemaView;
        assert_eq!(
            RefreshMaterializedView::new(&view).to_sql().sql(),
            r#"REFRESH MATERIALIZED VIEW "analytics"."user_stats""#
        );
        assert_eq!(
            RefreshMaterializedView::new(&view)
                .concurrently()
                .to_sql()
                .sql(),
            r#"REFRESH MATERIALIZED VIEW CONCURRENTLY "analytics"."user_stats""#
        );
    }

    #[test]
    fn test_helper_function() {
        let view = TestView;
        let refresh = refresh_materialized_view(&view);
        let sql = refresh.to_sql();

        assert_eq!(sql.sql(), r#"REFRESH MATERIALIZED VIEW "user_stats""#);
    }
}
