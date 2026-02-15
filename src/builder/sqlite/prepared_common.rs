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
                let (sql_str, params) = self.inner.bind(params);
                let params: Vec<$value> = params.map(Into::into).collect();

                conn.exec(sql_str, params).await
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
                let (sql_str, params) = self.inner.bind(params);
                let params: Vec<$value> = params.map(Into::into).collect();

                let mut rows = conn.fetch(sql_str, params).await?;

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
                let (sql_str, params) = self.inner.bind(params);
                let params: Vec<$value> = params.map(Into::into).collect();
                let mut rows = conn.fetch(sql_str, params).await?;

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
                let (sql_str, params) = self.inner.bind(params);
                let params: Vec<$value> = params.map(Into::into).collect();

                conn.exec(sql_str, params).await
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
                let (sql_str, params) = self.inner.bind(params);
                let params: Vec<$value> = params.map(Into::into).collect();
                let mut rows = conn.fetch(sql_str, params).await?;

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
                let (sql_str, params) = self.inner.bind(params);
                let params: Vec<$value> = params.map(Into::into).collect();
                let mut rows = conn.fetch(sql_str, params).await?;

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
