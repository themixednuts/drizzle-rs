macro_rules! sqlite_async_prepared_impl {
    ($conn:ty, $row:ty, $params_from_iter:path) => {
        impl<'a> PreparedStatement<'a> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute(
                &self,
                conn: &$conn,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, params) = self.inner.bind(params);

                conn.execute(sql_str, $params_from_iter(params))
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<T>(
                &self,
                conn: &$conn,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error:
                    Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params);

                let mut rows = conn.query(sql_str, $params_from_iter(params)).await?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await? {
                    let converted = T::try_from(&row).map_err(Into::into)?;
                    results.push(converted);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<T>(
                &self,
                conn: &$conn,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error:
                    Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params);
                let mut rows = conn.query(sql_str, $params_from_iter(params)).await?;

                if let Some(row) = rows.next().await? {
                    T::try_from(&row).map_err(Into::into)
                } else {
                    Err(drizzle_core::error::DrizzleError::NotFound)
                }
            }
        }

        impl OwnedPreparedStatement {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<'a>(
                &self,
                conn: &$conn,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, params) = self.inner.bind(params);

                conn.execute(sql_str, $params_from_iter(params))
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<'a, T>(
                &self,
                conn: &$conn,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error:
                    Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params);
                let mut rows = conn.query(sql_str, $params_from_iter(params)).await?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await? {
                    let converted = T::try_from(&row).map_err(Into::into)?;
                    results.push(converted);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<'a, T>(
                &self,
                conn: &$conn,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error:
                    Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params);
                let mut rows = conn.query(sql_str, $params_from_iter(params)).await?;

                if let Some(row) = rows.next().await? {
                    T::try_from(&row).map_err(Into::into)
                } else {
                    Err(drizzle_core::error::DrizzleError::NotFound)
                }
            }
        }
    };
}

pub(crate) use sqlite_async_prepared_impl;
