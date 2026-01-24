//! REFRESH MATERIALIZED VIEW query builder for PostgreSQL
//!
//! This module provides a builder for constructing `REFRESH MATERIALIZED VIEW` statements.
//!
//! # Examples
//!
//! ```ignore
//! use drizzle_postgres::builder::refresh::RefreshMaterializedView;
//!
//! // Basic refresh
//! let refresh = RefreshMaterializedView::new(&my_view);
//!
//! // Concurrent refresh (allows reads during refresh)
//! let refresh = RefreshMaterializedView::new(&my_view).concurrently();
//!
//! // Refresh without data (empties the view)
//! let refresh = RefreshMaterializedView::new(&my_view).with_no_data();
//! ```

use crate::values::PostgresValue;
use drizzle_core::traits::SQLViewInfo;
use drizzle_core::{SQL, ToSQL, Token};
use std::marker::PhantomData;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of RefreshMaterializedView
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshInitial;

/// Marker for the state after CONCURRENTLY is set
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshConcurrently;

/// Marker for the state after WITH NO DATA is set
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshWithNoData;

//------------------------------------------------------------------------------
// RefreshMaterializedView Builder
//------------------------------------------------------------------------------

/// Builder for REFRESH MATERIALIZED VIEW statements
///
/// PostgreSQL syntax:
/// ```sql
/// REFRESH MATERIALIZED VIEW [ CONCURRENTLY ] view_name [ WITH [ NO ] DATA ]
/// ```
///
/// Note: CONCURRENTLY and WITH NO DATA are mutually exclusive in PostgreSQL.
/// CONCURRENTLY requires the materialized view to have a unique index.
#[derive(Debug, Clone)]
pub struct RefreshMaterializedView<'a, State = RefreshInitial> {
    sql: SQL<'a, PostgresValue<'a>>,
    _state: PhantomData<State>,
}

impl<'a> RefreshMaterializedView<'a, RefreshInitial> {
    /// Creates a new REFRESH MATERIALIZED VIEW builder for the given view
    #[must_use]
    pub fn new<V: SQLViewInfo>(view: &'a V) -> Self {
        let schema = view.schema();
        let name = view.name();

        // Build: REFRESH MATERIALIZED VIEW "schema"."name"
        let sql = SQL::from_iter([Token::REFRESH, Token::MATERIALIZED, Token::VIEW])
            .append(SQL::ident(schema))
            .push(Token::DOT)
            .append(SQL::ident(name));

        Self {
            sql,
            _state: PhantomData,
        }
    }

    /// Adds the CONCURRENTLY option
    ///
    /// This allows the view to be refreshed without locking out concurrent reads.
    /// Requires the materialized view to have at least one unique index.
    ///
    /// Note: Cannot be combined with WITH NO DATA.
    #[must_use]
    pub fn concurrently(self) -> RefreshMaterializedView<'a, RefreshConcurrently> {
        // We need to insert CONCURRENTLY after VIEW
        // Current: REFRESH MATERIALIZED VIEW "schema"."name"
        // Desired: REFRESH MATERIALIZED VIEW CONCURRENTLY "schema"."name"

        // Get schema.name portion (last 3 chunks: ident, dot, ident)
        let chunks = self.sql.chunks;
        let schema_name_start = 3; // After REFRESH, MATERIALIZED, VIEW

        let mut new_sql = SQL::from_iter([
            Token::REFRESH,
            Token::MATERIALIZED,
            Token::VIEW,
            Token::CONCURRENTLY,
        ]);

        // Append the remaining chunks (schema.name)
        for chunk in chunks.into_iter().skip(schema_name_start) {
            new_sql = new_sql.push(chunk);
        }

        RefreshMaterializedView {
            sql: new_sql,
            _state: PhantomData,
        }
    }

    /// Adds the WITH NO DATA option
    ///
    /// This causes the materialized view to be emptied rather than refreshed with data.
    /// The view cannot be queried until data is added with a subsequent REFRESH.
    ///
    /// Note: Cannot be combined with CONCURRENTLY.
    #[must_use]
    pub fn with_no_data(self) -> RefreshMaterializedView<'a, RefreshWithNoData> {
        RefreshMaterializedView {
            sql: self.sql.push(Token::WITH).push(Token::NO).push(Token::DATA),
            _state: PhantomData,
        }
    }

    /// Adds the WITH DATA option (explicit, but this is the default behavior)
    #[must_use]
    pub fn with_data(self) -> Self {
        Self {
            sql: self.sql.push(Token::WITH).push(Token::DATA),
            _state: PhantomData,
        }
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

/// Creates a REFRESH MATERIALIZED VIEW statement for the given view
pub fn refresh_materialized_view<'a, V: SQLViewInfo>(
    view: &'a V,
) -> RefreshMaterializedView<'a, RefreshInitial> {
    RefreshMaterializedView::new(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock view for testing
    struct TestView;

    impl drizzle_core::traits::SQLTableInfo for TestView {
        fn name(&self) -> &str {
            "user_stats"
        }

        fn columns(&self) -> &'static [&'static dyn drizzle_core::traits::SQLColumnInfo] {
            &[]
        }

        fn dependencies(&self) -> &'static [&'static dyn drizzle_core::traits::SQLTableInfo] {
            &[]
        }
    }

    impl SQLViewInfo for TestView {
        fn definition_sql(&self) -> std::borrow::Cow<'static, str> {
            "SELECT * FROM users".into()
        }

        fn schema(&self) -> &'static str {
            "public"
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

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW "public"."user_stats""#
        );
    }

    #[test]
    fn test_concurrent_refresh() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view).concurrently();
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW CONCURRENTLY "public"."user_stats""#
        );
    }

    #[test]
    fn test_refresh_with_no_data() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view).with_no_data();
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW "public"."user_stats" WITH NO DATA"#
        );
    }

    #[test]
    fn test_refresh_with_data() {
        let view = TestView;
        let refresh = RefreshMaterializedView::new(&view).with_data();
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW "public"."user_stats" WITH DATA"#
        );
    }

    #[test]
    fn test_helper_function() {
        let view = TestView;
        let refresh = refresh_materialized_view(&view);
        let sql = refresh.to_sql();

        assert_eq!(
            sql.sql(),
            r#"REFRESH MATERIALIZED VIEW "public"."user_stats""#
        );
    }
}
