macro_rules! sqlite_async_prepared_impl {
    ($executor:path, $row:ty, $value:ty) => {
        impl<'a> PreparedStatement<'a> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute(
                &self,
                conn: &impl $executor,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));

                conn.exec(sql_str, driver_params).await
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<T>(
                &self,
                conn: &impl $executor,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));

                let mut rows = conn.fetch(sql_str, driver_params).await?;

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
                conn: &impl $executor,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));
                let mut rows = conn.fetch(sql_str, driver_params).await?;

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
                conn: &impl $executor,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));

                conn.exec(sql_str, driver_params).await
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<'a, T>(
                &self,
                conn: &impl $executor,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));
                let mut rows = conn.fetch(sql_str, driver_params).await?;

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
                conn: &impl $executor,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_sqlite::values::SQLiteValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));
                let mut rows = conn.fetch(sql_str, driver_params).await?;

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
