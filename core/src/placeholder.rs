use crate::expr::{Expr, NonNull, Scalar};
use crate::traits::{SQLParam, ToSQL};
use crate::{Param, SQL};
use core::fmt;

/// A SQL parameter placeholder.
///
/// Placeholders store a semantic name for parameter binding. The actual SQL syntax
/// (`$1`, `?`, `:name`) is determined by the `Dialect` at render time.
///
/// # Examples
/// ```ignore
/// // Named placeholder - rendered based on dialect
/// let placeholder = Placeholder::named("user_id");
///
/// // Anonymous placeholder - for positional parameters
/// let anon = Placeholder::anonymous();
/// ```
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct Placeholder {
    /// The semantic name of the parameter (used for binding by name).
    pub name: Option<&'static str>,
}

impl Placeholder {
    /// Creates a named placeholder.
    ///
    /// The name is used for binding; rendering is dialect-specific:
    /// - PostgreSQL: `$1`, `$2`, ... (positional, name ignored in SQL)
    /// - SQLite: `:name` for named placeholders
    /// - MySQL: `?` (positional, name ignored in SQL)
    pub const fn named(name: &'static str) -> Self {
        Placeholder { name: Some(name) }
    }

    /// Creates an anonymous placeholder (no name).
    ///
    /// Used for positional parameters where no name binding is needed.
    pub const fn anonymous() -> Self {
        Placeholder { name: None }
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Placeholder {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL {
            chunks: smallvec::smallvec![crate::SQLChunk::Param(Param {
                value: None,
                placeholder: *self,
            })],
        }
    }
}

impl<'a, V: SQLParam + 'a> Expr<'a, V> for Placeholder {
    type SQLType = crate::types::Placeholder;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl fmt::Display for Placeholder {
    /// Debug display: `?` for anonymous or `:name` for named.
    /// Note: actual SQL rendering uses dialect-specific placeholders via `SQL::write_to`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => write!(f, ":{}", name),
            None => write!(f, "?"),
        }
    }
}
