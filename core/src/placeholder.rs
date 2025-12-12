use crate::Dialect;
use core::fmt;

/// A SQL parameter placeholder.
///
/// Placeholders store a semantic name for parameter binding. The actual SQL syntax
/// (`$1`, `?`) is determined by the `Dialect` at render time.
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
    /// The actual SQL syntax is determined by the `Dialect` at render time:
    /// - PostgreSQL: `$1`, `$2`, ...
    /// - SQLite/MySQL: `?`
    pub const fn named(name: &'static str) -> Self {
        Placeholder { name: Some(name) }
    }

    /// Creates an anonymous placeholder (no name).
    ///
    /// Used for positional parameters where no name binding is needed.
    pub const fn anonymous() -> Self {
        Placeholder { name: None }
    }

    /// Renders this placeholder for the given dialect and 1-based index.
    #[inline]
    pub fn render(&self, dialect: Dialect, index: usize) -> String {
        dialect.render_placeholder(index)
    }

    /// Creates a new colon-style placeholder. Alias for `named()`.
    pub const fn colon(name: &'static str) -> Self {
        Self::named(name)
    }

    /// Creates a positional placeholder ('?'). Alias for `anonymous()`.
    pub const fn positional() -> Self {
        Self::anonymous()
    }
}

impl fmt::Display for Placeholder {
    /// Displays the placeholder as `?` for anonymous or `:name` for named.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => write!(f, ":{}", name),
            None => write!(f, "?"),
        }
    }
}
