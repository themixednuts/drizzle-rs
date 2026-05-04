macro_rules! sqlite_async_prepared_impl {
    ($executor:path, $row:ty, $value:ty) => {
        impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<u64> {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));

                conn.exec(sql_str, driver_params).await
            }

            /// Runs the prepared statement and returns all matching rows
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
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));

                let mut rows = conn.fetch(sql_str, driver_params).await?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await? {
                    let converted = <Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?;
                    results.push(converted);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<T, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<T>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));
                let mut rows = conn.fetch(sql_str, driver_params).await?;

                if let Some(row) = rows.next().await? {
                    <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)
                } else {
                    Err(drizzle_core::error::DrizzleError::NotFound)
                }
            }
        }

        impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<'a, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<u64> {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));

                conn.exec(sql_str, driver_params).await
            }

            /// Runs the prepared statement and returns all matching rows
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
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));
                let mut rows = conn.fetch(sql_str, driver_params).await?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await? {
                    let converted = <Marker as drizzle_core::row::DecodeSelectedRef<
                        &$row,
                        T,
                    >>::decode(&row)?;
                    results.push(converted);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<'a, T, const N: usize>(
                &self,
                conn: &impl $executor,
                params: [drizzle_core::param::ParamBind<
                    'a,
                    drizzle_sqlite::values::SQLiteValue<'a>,
                >; N],
            ) -> drizzle_core::error::Result<T>
            where
                for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r $row, T>,
            {
                debug_assert_eq!(N, self.inner.external_param_count(), "parameter count mismatch: expected {} params but got {}", self.inner.external_param_count(), N);
                let (sql_str, params) = self.inner.bind(params)?;
                let mut driver_params = Vec::with_capacity(self.inner.params.len());
                driver_params.extend(params.map(Into::into));
                let mut rows = conn.fetch(sql_str, driver_params).await?;

                if let Some(row) = rows.next().await? {
                    <Marker as drizzle_core::row::DecodeSelectedRef<&$row, T>>::decode(&row)
                } else {
                    Err(drizzle_core::error::DrizzleError::NotFound)
                }
            }
        }
    };
}

pub(crate) use sqlite_async_prepared_impl;
