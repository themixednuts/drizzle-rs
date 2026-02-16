/// Shared decoded row cursor used by sqlite backends.
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

    pub fn next(&mut self) -> drizzle_core::error::Result<Option<R>> {
        Ok(self.rows.next())
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
    pub(crate) fn new(rows: libsql::Rows) -> Self {
        Self {
            rows,
            _marker: core::marker::PhantomData,
        }
    }

    pub async fn next(&mut self) -> drizzle_core::error::Result<Option<R>> {
        match self
            .rows
            .next()
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))?
        {
            Some(row) => Ok(Some(R::try_from(&row).map_err(Into::into)?)),
            None => Ok(None),
        }
    }

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
    pub(crate) fn new(rows: turso::Rows) -> Self {
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

    pub async fn next(&mut self) -> drizzle_core::error::Result<Option<R>> {
        let row = self.rows.next().await.map_err(|e| {
            if let Some(sql) = &self.sql {
                drizzle_core::error::DrizzleError::ExecutionError(
                    format!("{}\n\nSQL: {}", e, sql).into(),
                )
            } else {
                drizzle_core::error::DrizzleError::Other(e.to_string().into())
            }
        })?;

        match row {
            Some(row) => Ok(Some(R::try_from(&row).map_err(Into::into)?)),
            None => Ok(None),
        }
    }

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
