//! Naming rules every dialect's schema producers share.

/// The columns that name a table-level (composite) foreign key: its first
/// column, or all of them when another foreign key on the same table would
/// otherwise get the same name.
#[must_use]
pub fn composite_foreign_key_name_columns<'a>(
    columns: &'a [&'a str],
    collides: bool,
) -> &'a [&'a str] {
    if collides || columns.is_empty() {
        columns
    } else {
        &columns[..1]
    }
}
