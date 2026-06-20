#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use drizzle::core::expr::{and, eq};
use drizzle::error::DrizzleError;
use drizzle::postgres::prelude::*;
use drizzle::postgres::traits::DrizzlePostgresColumn;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct PgU32Be(u32);

impl PgU32Be {
    fn from_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        let bytes: [u8; 4] = value.try_into().map_err(|_| {
            DrizzleError::ConversionError(
                format!("expected 4 bytes for PgU32Be, got {}", value.len()).into(),
            )
        })?;
        Ok(Self(u32::from_be_bytes(bytes)))
    }
}

impl DrizzlePostgresColumn for PgU32Be {
    type SQLType = drizzle::postgres::types::Bytea;

    const SQL_TYPE: &'static str = "BYTEA";

    fn decode(row: &drizzle::postgres::Row, idx: usize) -> Result<Self, DrizzleError> {
        let bytes: Vec<u8> = row.get::<_, Vec<u8>>(idx);
        Self::from_bytes(&bytes)
    }

    fn encode(&self) -> PostgresValue<'_> {
        PostgresValue::Bytea(Cow::Owned(self.0.to_be_bytes().to_vec()))
    }

    fn encode_owned(self) -> OwnedPostgresValue {
        OwnedPostgresValue::Bytea(self.0.to_be_bytes().to_vec())
    }
}

#[cfg(feature = "aws-data-api")]
impl drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row> for PgU32Be {
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(
        row: &drizzle::postgres::aws_data_api::Row,
        offset: usize,
    ) -> Result<Self, DrizzleError> {
        let bytes = <Vec<u8> as drizzle::core::FromDrizzleRow<
            drizzle::postgres::aws_data_api::Row,
        >>::from_row_at(row, offset)?;
        Self::from_bytes(&bytes)
    }
}

#[test]
fn custom_postgres_column_from_bytes_rejects_invalid_length() {
    assert!(PgU32Be::from_bytes(&[1, 2, 3]).is_err());
}

#[test]
fn custom_postgres_column_encode_uses_bytea() {
    let PostgresValue::Bytea(bytes) = PgU32Be(0x0a0b_0c0d).encode() else {
        panic!("PgU32Be should encode as BYTEA");
    };

    assert_eq!(bytes.as_ref(), &[0x0a, 0x0b, 0x0c, 0x0d]);
}

#[PostgresTable(name = "pg_custom_u32_be_test")]
struct PgCustomU32BeTest {
    #[column(primary, serial)]
    id: i32,
    payload: PgU32Be,
}

#[derive(PostgresSchema)]
struct PgCustomU32BeSchema {
    table: PgCustomU32BeTest,
}

struct PgCustomU32BeAlias;

impl drizzle::core::Tag for PgCustomU32BeAlias {
    const NAME: &'static str = "pg_custom_u32_alias";
}

#[drizzle::test]
fn custom_postgres_column_roundtrip_and_metadata(db: &mut TestDb<PgCustomU32BeSchema>) {
    let PgCustomU32BeSchema { table } = schema;

    assert!(PgCustomU32BeTest::ddl_sql().contains("\"payload\" BYTEA NOT NULL"));
    assert_eq!(
        <PgCustomU32BeTest as drizzle::core::DrizzleTable>::TABLE_REF.columns[1].sql_type,
        "BYTEA"
    );

    let value = PgU32Be(0x0a0b_0c0d);
    db.insert(table)
        .values([InsertPgCustomU32BeTest::new(value)])
        .execute();

    let rows: Vec<SelectPgCustomU32BeTest> = db
        .select((table.id, table.payload))
        .from(table)
        .r#where(and(eq(table.payload, value), eq(table.payload, &value)))
        .all();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload, value);

    let alias = PgCustomU32BeTest::alias::<PgCustomU32BeAlias>();
    let rows: Vec<SelectPgCustomU32BeTest> = db
        .select((alias.id, alias.payload))
        .from(alias)
        .r#where(eq(alias.payload, value))
        .all();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload, value);
}
