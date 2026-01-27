macro_rules! postgres_prepared_sync_impl {
    ($client:ty, $row:ty, $to_sql:path) => {
        impl<'a> PreparedStatement<'a> {
            /// Runs the prepared statement and returns the number of affected rows
            pub fn execute(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'a>> =
                    bound_params.collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                client.execute(sql_str, &params_refs).map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub fn all<T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'a>> =
                    bound_params.collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let rows = client.query(sql_str, &params_refs)?;

                let mut results = Vec::with_capacity(rows.len());
                for row in &rows {
                    results.push(T::try_from(row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub fn get<T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'a>> =
                    bound_params.collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let row = client.query_one(sql_str, &params_refs)?;
                T::try_from(&row).map_err(Into::into)
            }
        }

        impl OwnedPreparedStatement {
            /// Runs the prepared statement and returns the number of affected rows
            pub fn execute<'a>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'_>> = bound_params
                    .map(drizzle_postgres::values::PostgresValue::from)
                    .collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                client.execute(sql_str, &params_refs).map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub fn all<'a, T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'_>> = bound_params
                    .map(drizzle_postgres::values::PostgresValue::from)
                    .collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let rows = client.query(sql_str, &params_refs)?;

                let mut results = Vec::with_capacity(rows.len());
                for row in &rows {
                    results.push(T::try_from(row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub fn get<'a, T>(
                &self,
                client: &mut $client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'_>> = bound_params
                    .map(drizzle_postgres::values::PostgresValue::from)
                    .collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let row = client.query_one(sql_str, &params_refs)?;
                T::try_from(&row).map_err(Into::into)
            }
        }
    };
}

macro_rules! postgres_prepared_async_impl {
    ($client:ty, $row:ty, $to_sql:path) => {
        impl<'a> PreparedStatement<'a> {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'a>> =
                    bound_params.collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                client
                    .execute(sql_str, &params_refs)
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'a>> =
                    bound_params.collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let rows = client.query(sql_str, &params_refs).await?;

                let mut results = Vec::with_capacity(rows.len());
                for row in &rows {
                    results.push(T::try_from(row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'a>> =
                    bound_params.collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let row = client.query_one(sql_str, &params_refs).await?;
                T::try_from(&row).map_err(Into::into)
            }
        }

        impl OwnedPreparedStatement {
            /// Runs the prepared statement and returns the number of affected rows
            pub async fn execute<'a>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<u64> {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'_>> = bound_params
                    .map(drizzle_postgres::values::PostgresValue::from)
                    .collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                client
                    .execute(sql_str, &params_refs)
                    .await
                    .map_err(Into::into)
            }

            /// Runs the prepared statement and returns all matching rows
            pub async fn all<'a, T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<Vec<T>>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'_>> = bound_params
                    .map(drizzle_postgres::values::PostgresValue::from)
                    .collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let rows = client.query(sql_str, &params_refs).await?;

                let mut results = Vec::with_capacity(rows.len());
                for row in &rows {
                    results.push(T::try_from(row).map_err(Into::into)?);
                }

                Ok(results)
            }

            /// Runs the prepared statement and returns a single row
            pub async fn get<'a, T>(
                &self,
                client: &$client,
                params: impl IntoIterator<
                    Item = drizzle_core::param::ParamBind<
                        'a,
                        drizzle_postgres::values::PostgresValue<'a>,
                    >,
                >,
            ) -> drizzle_core::error::Result<T>
            where
                T: for<'r> TryFrom<&'r $row>,
                for<'r> <T as TryFrom<&'r $row>>::Error: Into<drizzle_core::error::DrizzleError>,
            {
                let (sql_str, bound_params) = self.inner.bind(params);
                let params_vec: Vec<drizzle_postgres::values::PostgresValue<'_>> = bound_params
                    .map(drizzle_postgres::values::PostgresValue::from)
                    .collect();
                let params_refs: Vec<&(dyn $to_sql + Sync)> = params_vec
                    .iter()
                    .map(|p| p as &(dyn $to_sql + Sync))
                    .collect();

                let row = client.query_one(sql_str, &params_refs).await?;
                T::try_from(&row).map_err(Into::into)
            }
        }
    };
}

pub(crate) use postgres_prepared_async_impl;
pub(crate) use postgres_prepared_sync_impl;
