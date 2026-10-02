/// Rows returned by `.rows()` on the rusqlite driver.
///
/// Every row was fetched and decoded before the query returned, so iterating
/// never touches the database and every `Result<R>` item is `Ok`.
pub struct Rows<R> {
    #[cfg(feature = "std")]
    rows: std::vec::IntoIter<R>,
    #[cfg(not(feature = "std"))]
    rows: alloc::vec::IntoIter<R>,
}

impl<R> Rows<R> {
    #[cfg(feature = "std")]
    pub(crate) fn new(rows: Vec<R>) -> Self {
        Self {
            rows: rows.into_iter(),
        }
    }

    #[cfg(not(feature = "std"))]
    pub(crate) fn new(rows: alloc::vec::Vec<R>) -> Self {
        Self {
            rows: rows.into_iter(),
        }
    }
}

impl<R> Iterator for Rows<R> {
    type Item = drizzle_core::error::Result<R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows.next().map(Ok)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rows.size_hint()
    }
}

impl<R> ExactSizeIterator for Rows<R> {}

/// Rows returned by `.rows()` on the libsql driver, fetched and decoded one
/// at a time.
///
/// Read them with [`next`](Self::next) or gather them with
/// [`collect`](Self::collect).
#[cfg(feature = "libsql")]
pub struct LibsqlRows<R> {
    rows: libsql::Rows,
    _marker: core::marker::PhantomData<R>,
}

#[cfg(feature = "libsql")]
impl<R> LibsqlRows<R>
where
    R: for<'r> TryFrom<&'r libsql::Row>,
    for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
{
    pub(crate) const fn new(rows: libsql::Rows) -> Self {
        Self {
            rows,
            _marker: core::marker::PhantomData,
        }
    }

    /// Fetches and decodes the next row, or returns `None` after the last one.
    ///
    /// # Errors
    ///
    /// Returns an error when fetching the row fails or it cannot be decoded
    /// into `R`.
    pub async fn next(&mut self) -> drizzle_core::error::Result<Option<R>> {
        match self
            .rows
            .next()
            .await
            .map_err(drizzle_core::error::DrizzleError::from)?
        {
            Some(row) => Ok(Some(R::try_from(&row).map_err(Into::into)?)),
            None => Ok(None),
        }
    }

    /// Fetches and decodes every remaining row into `C`, such as a `Vec<R>`.
    ///
    /// # Errors
    ///
    /// Returns the first error from [`next`](Self::next).
    pub async fn collect<C>(mut self) -> drizzle_core::error::Result<C>
    where
        C: Default + Extend<R>,
    {
        let mut results = C::default();
        while let Some(row) = self.next().await? {
            results.extend(::core::iter::once(row));
        }
        Ok(results)
    }
}

/// Rows returned by `.rows()` on the turso driver, fetched and decoded one
/// at a time.
///
/// Read them with [`next`](Self::next) or gather them with
/// [`collect`](Self::collect).
#[cfg(feature = "turso")]
pub struct TursoRows<R> {
    rows: turso::Rows,
    sql: Option<Box<str>>,
    _marker: core::marker::PhantomData<R>,
}

#[cfg(feature = "turso")]
impl<R> TursoRows<R>
where
    R: for<'r> TryFrom<&'r turso::Row>,
    for<'r> <R as TryFrom<&'r turso::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
{
    pub(crate) const fn new(rows: turso::Rows) -> Self {
        Self {
            rows,
            sql: None,
            _marker: core::marker::PhantomData,
        }
    }

    pub(crate) fn with_sql(rows: turso::Rows, sql: impl Into<Box<str>>) -> Self {
        Self {
            rows,
            sql: Some(sql.into()),
            _marker: core::marker::PhantomData,
        }
    }

    /// Fetches and decodes the next row, or returns `None` after the last one.
    ///
    /// # Errors
    ///
    /// Returns an error when fetching the row fails or it cannot be decoded
    /// into `R`.
    pub async fn next(&mut self) -> drizzle_core::error::Result<Option<R>> {
        let row = self.rows.next().await.map_err(|e| {
            let source = drizzle_core::error::DrizzleError::from(e);
            match self.sql.as_ref() {
                Some(sql) => drizzle_core::error::DrizzleError::QueryFailed {
                    ctx: Box::new(drizzle_core::error::QueryContext {
                        sql: sql.as_ref().into(),
                        params: Box::default(),
                        param_count: 0,
                    }),
                    source: Box::new(source),
                },
                None => source,
            }
        })?;

        match row {
            Some(row) => Ok(Some(R::try_from(&row).map_err(Into::into)?)),
            None => Ok(None),
        }
    }

    /// Fetches and decodes every remaining row into `C`, such as a `Vec<R>`.
    ///
    /// # Errors
    ///
    /// Returns the first error from [`next`](Self::next).
    pub async fn collect<C>(mut self) -> drizzle_core::error::Result<C>
    where
        C: Default + Extend<R>,
    {
        let mut results = C::default();
        while let Some(row) = self.next().await? {
            results.extend(::core::iter::once(row));
        }
        Ok(results)
    }
}
