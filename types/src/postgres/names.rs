//! Default names for `PostgreSQL` constraints the schema producers derive.

use crate::alloc_prelude::{String, format};

pub use crate::names::composite_foreign_key_name_columns;

/// The default name of a foreign key on `table` over `columns`:
/// `{table}_{columns joined by _}_fkey`.
///
/// Pass one column for a column-level key, and
/// [`composite_foreign_key_name_columns`] for a table-level one, so two keys
/// on one table never share a name.
///
/// # Examples
///
/// ```
/// use drizzle_types::postgres::names::{composite_foreign_key_name_columns, foreign_key_name};
///
/// assert_eq!(foreign_key_name("posts", &["author_id"]), "posts_author_id_fkey");
/// // `(tenant_id, user_id)` next to a key on `tenant_id` alone.
/// let columns = composite_foreign_key_name_columns(&["tenant_id", "user_id"], true);
/// assert_eq!(foreign_key_name("posts", columns), "posts_tenant_id_user_id_fkey");
/// ```
#[must_use]
pub fn foreign_key_name(table: &str, columns: &[&str]) -> String {
    format!("{table}_{}_fkey", columns.join("_"))
}
