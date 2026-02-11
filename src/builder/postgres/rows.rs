use std::marker::PhantomData;

use drizzle_core::error::DrizzleError;

/// Shared lazy decoded row cursor used by postgres drivers.
pub struct DecodeRows<RowT, R> {
    rows: std::vec::IntoIter<RowT>,
    _marker: PhantomData<R>,
}

impl<RowT, R> DecodeRows<RowT, R> {
    pub(crate) fn new(rows: Vec<RowT>) -> Self {
        Self {
            rows: rows.into_iter(),
            _marker: PhantomData,
        }
    }
}

impl<RowT, R> DecodeRows<RowT, R>
where
    R: for<'r> TryFrom<&'r RowT>,
    for<'r> <R as TryFrom<&'r RowT>>::Error: Into<DrizzleError>,
{
    pub fn next(&mut self) -> drizzle_core::error::Result<Option<R>> {
        match self.rows.next() {
            Some(row) => Ok(Some(R::try_from(&row).map_err(Into::into)?)),
            None => Ok(None),
        }
    }
}

impl<RowT, R> Iterator for DecodeRows<RowT, R>
where
    R: for<'r> TryFrom<&'r RowT>,
    for<'r> <R as TryFrom<&'r RowT>>::Error: Into<DrizzleError>,
{
    type Item = drizzle_core::error::Result<R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows
            .next()
            .map(|row| R::try_from(&row).map_err(Into::into))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rows.size_hint()
    }
}

impl<RowT, R> ExactSizeIterator for DecodeRows<RowT, R>
where
    R: for<'r> TryFrom<&'r RowT>,
    for<'r> <R as TryFrom<&'r RowT>>::Error: Into<DrizzleError>,
{
}
