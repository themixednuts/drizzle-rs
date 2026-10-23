macro_rules! sqlite_async_prepared_impl {
    ($executor:path, $row:ty, $value:ty) => {
        impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
            /// Binds `params` and runs the statement, returning the number of rows it
            /// changed.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra),
            /// or the database error when the statement fails.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn execute<const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, params) = self.inner.bind(params)?;
                let params: smallvec::SmallVec<[_; 8]> = params.collect();
                let mut driver_params = Vec::with_capacity(params.len());
                driver_params.extend(params.iter().map(Into::into));
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params.iter().collect::<Vec<_>>(),
                    )
                };

                conn.exec(sql_str, driver_params).await.with_query(context)
            }

            /// Binds `params`, runs the query, and decodes every row into `T`.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra),
            /// the database error when the query fails, or a decode error when a row
            /// does not fit `T`.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn all<T, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let params: smallvec::SmallVec<[_; 8]> = params.collect();
                let mut driver_params = Vec::with_capacity(params.len());
                driver_params.extend(params.iter().map(Into::into));
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params.iter().collect::<Vec<_>>(),
                    )
                };

                let mut rows = conn.fetch(sql_str, driver_params).await.with_query(context)?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await.with_query(context)? {
                    let converted = <Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?;
                    results.push(converted);
                }

                Ok(results)
            }

            /// Binds `params`, runs the query, and decodes its first row into `T`.
            ///
            /// Returns `None` when no row matches.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra), the database error when
            /// the query fails, or a decode error when the row does not fit `T`.
            ///
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn get<T, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<Option<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let params: smallvec::SmallVec<[_; 8]> = params.collect();
                let mut driver_params = Vec::with_capacity(params.len());
                driver_params.extend(params.iter().map(Into::into));
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params.iter().collect::<Vec<_>>(),
                    )
                };
                let mut rows = conn.fetch(sql_str, driver_params).await.with_query(context)?;

                let decoded = if let Some(row) = rows.next().await.with_query(context)? {
                    <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row).map(Some)
                } else {
                    Ok(None)
                };
                while rows.next().await.with_query(context)?.is_some() {}
                decoded
            }
        }

        impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
            /// Binds `params` and runs the statement, returning the number of rows it
            /// changed.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra),
            /// or the database error when the statement fails.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn execute<'a, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, params) = self.inner.bind(params)?;
                let params: smallvec::SmallVec<[_; 8]> = params.collect();
                let mut driver_params = Vec::with_capacity(params.len());
                driver_params.extend(params.iter().map(Into::into));
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params.iter().collect::<Vec<_>>(),
                    )
                };

                conn.exec(sql_str, driver_params).await.with_query(context)
            }

            /// Binds `params`, runs the query, and decodes every row into `T`.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra),
            /// the database error when the query fails, or a decode error when a row
            /// does not fit `T`.
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn all<'a, T, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let params: smallvec::SmallVec<[_; 8]> = params.collect();
                let mut driver_params = Vec::with_capacity(params.len());
                driver_params.extend(params.iter().map(Into::into));
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params.iter().collect::<Vec<_>>(),
                    )
                };
                let mut rows = conn.fetch(sql_str, driver_params).await.with_query(context)?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await.with_query(context)? {
                    let converted = <Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?;
                    results.push(converted);
                }

                Ok(results)
            }

            /// Binds `params`, runs the query, and decodes its first row into `T`.
            ///
            /// Returns `None` when no row matches.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
            /// statement's placeholders (missing, duplicated, or extra), the database error when
            /// the query fails, or a decode error when the row does not fit `T`.
            ///
            ///
            /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
            ///
            /// # Panics
            ///
            /// In debug builds, panics when `N` differs from the number of placeholders.
            pub async fn get<'a, T, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<Option<T>>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let params: smallvec::SmallVec<[_; 8]> = params.collect();
                let mut driver_params = Vec::with_capacity(params.len());
                driver_params.extend(params.iter().map(Into::into));
                let context = || {
                    drizzle_core::error::QueryContext::new(
                        sql_str,
                        &params.iter().collect::<Vec<_>>(),
                    )
                };
                let mut rows = conn.fetch(sql_str, driver_params).await.with_query(context)?;

                let decoded = if let Some(row) = rows.next().await.with_query(context)? {
                    <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row).map(Some)
                } else {
                    Ok(None)
                };
                while rows.next().await.with_query(context)?.is_some() {}
                decoded
            }
        }
    };
}

pub(crate) use sqlite_async_prepared_impl;
