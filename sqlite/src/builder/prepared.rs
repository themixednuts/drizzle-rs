use crate::prelude::*;

use drizzle_core::{
    OwnedParam, Param,
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
};

use crate::values::{OwnedSQLiteValue, SQLiteValue};

/// A rendered `SQLite` statement split into SQL text and parameter slots.
///
/// Placeholders (see `drizzle_core::Placeholder`) stay as empty slots that
/// are filled when the statement runs; other values are already bound. This
/// type borrows its values for `'a`; use [`into_owned`](Self::into_owned)
/// or [`OwnedPreparedStatement`] to keep it longer. Its `Display` output is
/// the SQL text.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub(crate) inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
}

impl PreparedStatement<'_> {
    /// Clones this statement into an [`OwnedPreparedStatement`] with no
    /// borrowed data.
    #[must_use]
    pub fn into_owned(&self) -> OwnedPreparedStatement {
        let owned_params = self.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: owned_params.collect::<Box<[_]>>(),
            sql: self.inner.sql.clone(),
        };

        OwnedPreparedStatement { inner }
    }
}

/// A [`PreparedStatement`] that owns all of its values.
///
/// It has no lifetime, so it can be cached or stored in application state.
/// Convert with [`PreparedStatement::into_owned`] or
/// `From<PreparedStatement>`, and back with `From<OwnedPreparedStatement>`.
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub(crate) inner: CoreOwnedPreparedStatement<crate::values::OwnedSQLiteValue>,
}
impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        let owned_params = value.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });
        let inner = CoreOwnedPreparedStatement {
            text_segments: value.inner.text_segments,
            params: owned_params.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        Self { inner }
    }
}

impl From<OwnedPreparedStatement> for PreparedStatement<'_> {
    fn from(value: OwnedPreparedStatement) -> Self {
        let sqlitevalue = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(SQLiteValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: sqlitevalue.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        PreparedStatement { inner }
    }
}

impl OwnedPreparedStatement {}

impl core::fmt::Display for PreparedStatement<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl core::fmt::Display for OwnedPreparedStatement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::SQLiteValue;
    use drizzle_core::{SQL, ToSQL, prepared::prepare_render};

    #[test]
    fn test_prepare_render_basic() {
        // Test the basic prepare_render functionality for SQLite
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT * FROM users WHERE id = ")
            .append(drizzle_core::Placeholder::named("user_id").to_sql())
            .append(SQL::raw(" AND name = "))
            .append(drizzle_core::Placeholder::named("user_name").to_sql());

        let prepared = prepare_render(&sql);

        // Should have 3 text segments: before first param, between params, after last param
        assert_eq!(prepared.text_segments.len(), 3);
        assert_eq!(prepared.params.len(), 2);

        // Verify text segments contain expected content
        assert!(prepared.text_segments[0].contains("SELECT * FROM users WHERE id"));
        assert!(prepared.text_segments[1].contains("AND name"));
    }

    #[test]
    fn test_prepare_with_no_parameters() {
        // Test preparing SQL with no parameters
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT COUNT(*) FROM users");
        let prepared = prepare_render(&sql);

        assert_eq!(prepared.text_segments.len(), 1);
        assert_eq!(prepared.params.len(), 0);
        assert_eq!(prepared.text_segments[0], "SELECT COUNT(*) FROM users");
    }

    #[test]
    fn test_prepared_statement_display() {
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT * FROM users")
            .append(SQL::raw(" WHERE id = "))
            .append(drizzle_core::Placeholder::named("id").to_sql());

        let prepared = prepare_render(&sql);
        let display = format!("{}", prepared);

        assert!(display.contains("SELECT * FROM users"));
        assert!(display.contains("WHERE id"));
    }

    #[test]
    fn test_owned_conversion_roundtrip() {
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT name FROM users WHERE id = ")
            .append(drizzle_core::Placeholder::named("id").to_sql());

        let prepared = prepare_render(&sql);
        let core_prepared = PreparedStatement { inner: prepared };

        // Convert to owned
        let owned = core_prepared.into_owned();

        // Convert back to borrowed
        let borrowed: PreparedStatement<'_> = owned.into();

        // Verify structure is preserved
        assert_eq!(borrowed.inner.text_segments.len(), 2);
        assert_eq!(borrowed.inner.params.len(), 1);
    }
}
