use drizzle_core::SQLColumn;

use crate::values::MySQLValue;

/// A generated column belonging to a MySQL table.
pub trait MySQLColumn<'a>: SQLColumn<'a, MySQLValue<'a>> {
    /// Backtick-quoted column identifier for const DDL generation.
    const DDL_NAME: &'static str;

    /// Whether the column uses MySQL's `AUTO_INCREMENT` behavior.
    const AUTO_INCREMENT: bool = false;

    /// Effective character set declared by the column or its table.
    const CHARSET: Option<&'static str> = None;

    /// Effective collation declared by the column or its table.
    const COLLATE: Option<&'static str> = None;
}

/// Marker for a column that MySQL can index directly.
///
/// `TEXT` and `BLOB` families require a prefix length, while `JSON` requires an
/// indexed generated scalar column. Those capabilities are intentionally kept
/// out of this marker.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used directly in a MySQL index",
    note = "use a bounded character/binary column, or index a generated scalar column for JSON"
)]
pub trait MySQLIndexColumn {}

impl<'a, T> MySQLColumn<'a> for &T
where
    T: MySQLColumn<'a>,
    for<'r> &'r T: SQLColumn<'a, MySQLValue<'a>>,
{
    const DDL_NAME: &'static str = T::DDL_NAME;
    const AUTO_INCREMENT: bool = T::AUTO_INCREMENT;
    const CHARSET: Option<&'static str> = T::CHARSET;
    const COLLATE: Option<&'static str> = T::COLLATE;
}

#[doc(hidden)]
pub const fn optional_str_eq(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => str_eq(left, right),
        _ => false,
    }
}

const fn str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
