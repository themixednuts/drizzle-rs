//! PostgreSQL JSONB column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL JSONB columns.
///
/// JSONB stores JSON data in a decomposed binary format. Unlike JSON, JSONB
/// supports indexing and is generally more efficient for querying.
///
/// See: <https://www.postgresql.org/docs/current/datatype-json.html>
///
/// # JSON vs JSONB
///
/// - `JSON` - Stores exact copy of input, preserves whitespace and key order
/// - `JSONB` - Decomposed binary storage, faster to process, supports indexing
///
/// **Recommendation:** Use JSONB unless you need exact input preservation.
#[derive(Debug, Clone, Copy)]
pub struct JsonbBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> JsonbBuilder<T> {
    /// Creates a new JSONB column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_not_null: false,
            has_default: false,
        }
    }

    /// Adds a NOT NULL constraint to this column.
    #[inline]
    pub const fn not_null(self) -> Self {
        Self {
            is_not_null: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[jsonb(default = "{}")]
    /// metadata: serde_json::Value,  // SQL: metadata JSONB DEFAULT '{}'
    /// ```
    #[inline]
    pub const fn default(self, _value: &'static str) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate default values at runtime.
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a JSONB column builder.
///
/// JSONB stores JSON data in binary format with indexing support.
///
/// See: <https://www.postgresql.org/docs/current/datatype-json.html>
#[inline]
pub const fn jsonb<T>() -> JsonbBuilder<T> {
    JsonbBuilder::new()
}

/// Creates a JSON column builder (uses JSONB builder since they share the same API).
///
/// JSON stores JSON data as exact text input.
///
/// See: <https://www.postgresql.org/docs/current/datatype-json.html>
#[inline]
pub const fn json<T>() -> JsonbBuilder<T> {
    JsonbBuilder::new()
}
