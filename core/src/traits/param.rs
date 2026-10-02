use crate::dialect::Dialect;

/// A driver's value type: what bound parameters hold (`SQLiteValue`,
/// `PostgresValue`, `MySQLValue`, ...).
///
/// Every [`SQL`](crate::SQL) fragment is generic over one, and it fixes the
/// dialect: placeholder syntax, identifier quoting, and type mappings.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SQL parameter type",
    label = "use a dialect-specific value type (e.g., SQLiteValue, PostgresValue)"
)]
pub trait SQLParam: Clone + core::fmt::Debug {
    /// The dialect, at runtime.
    const DIALECT: Dialect;

    /// The dialect, as a type ([`SQLiteDialect`](crate::SQLiteDialect),
    /// [`PostgresDialect`](crate::PostgresDialect) or
    /// [`MySQLDialect`](crate::MySQLDialect)).
    ///
    /// Selects type mappings ([`SQLTypeToRust`](crate::row::SQLTypeToRust),
    /// [`DialectTypes`](crate::dialect::DialectTypes)) and dialect-only
    /// features ([`DialectSupports`](crate::DialectSupports)).
    type DialectMarker: crate::dialect::DialectTypes;

    /// Converts a `LIMIT`/`OFFSET` value into a parameter, or returns `None`
    /// to write it as a literal (the default).
    ///
    /// Binding keeps the SQL text the same across pages, so a cached
    /// prepared statement can be reused.
    #[inline]
    #[must_use]
    fn pagination_param(value: usize) -> Option<Self> {
        let _ = value;
        None
    }

    /// Appends this value to `buf` as a SQL literal of this dialect.
    ///
    /// Used where a statement cannot take bound parameters, such as the body
    /// of a `CREATE VIEW`. Returns `false`, having written nothing, when the
    /// value has no literal form; the default knows none.
    #[inline]
    fn write_literal(&self, buf: &mut crate::prelude::String) -> bool {
        let _ = buf;
        false
    }
}

// Implement SQLParam for common types
// impl<T: SQLParam> SQLParam for Option<T> {}
// impl<T: SQLParam> SQLParam for Vec<T> {}
// impl<T: SQLParam> SQLParam for Box<[T]> {}
// impl<T: SQLParam> SQLParam for Rc<T> {}
// impl<T: SQLParam> SQLParam for Arc<T> {}
// impl<T: SQLParam> SQLParam for RefCell<T> {}
// impl<'a, T: SQLParam> SQLParam for Cow<'a, T> {}
// impl<T: SQLParam> SQLParam for &[T] {}
// impl<T: SQLParam> SQLParam for &T {}
// impl<const N: usize, T: SQLParam> SQLParam for [T; N] {}
