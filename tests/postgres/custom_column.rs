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

// `Option<PgU32Be>` columns decode through the `Option<T: NullProbeRow>` impl.
#[cfg(feature = "aws-data-api")]
impl drizzle::core::NullProbeRow<drizzle::postgres::aws_data_api::Row> for PgU32Be {
    fn is_null_at(
        row: &drizzle::postgres::aws_data_api::Row,
        offset: usize,
    ) -> Result<bool, DrizzleError> {
        drizzle::postgres::aws_data_api::is_null_at(row, offset)
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

#[PostgresTable(name = "pg_custom_join_owner")]
struct PgCustomJoinOwner {
    #[column(primary)]
    id: i32,
}

// The custom BYTEA column comes first: `Option<SelectPgCustomJoinTag>` probes
// every column, whatever its wire type, to tell a LEFT JOIN miss from a match.
#[PostgresTable(name = "pg_custom_join_tag")]
struct PgCustomJoinTag {
    payload: PgU32Be,
    #[column(primary)]
    owner_id: i32,
}

#[derive(PostgresSchema)]
struct PgCustomJoinSchema {
    owner: PgCustomJoinOwner,
    tag: PgCustomJoinTag,
}

#[drizzle::test]
fn custom_postgres_column_first_in_left_join(db: &mut TestDb<PgCustomJoinSchema>) {
    let PgCustomJoinSchema { owner, tag } = schema;

    db.insert(owner)
        .values([
            InsertPgCustomJoinOwner::new(1),
            InsertPgCustomJoinOwner::new(2),
        ])
        .execute();
    db.insert(tag)
        .values([InsertPgCustomJoinTag::new(PgU32Be(7), 1)])
        .execute();

    let rows: Vec<(SelectPgCustomJoinOwner, Option<SelectPgCustomJoinTag>)> = db
        .select(())
        .from(owner)
        .left_join((tag, eq(tag.owner_id, owner.id)))
        .order_by(owner.id)
        .all();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1.as_ref().map(|t| t.payload), Some(PgU32Be(7)));
    assert!(rows[1].1.is_none());
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

#[PostgresTable(name = "pg_custom_nullable_test")]
struct PgCustomNullableTest {
    #[column(primary, serial)]
    id: i32,
    payload: Option<PgU32Be>,
}

#[derive(PostgresSchema)]
struct PgCustomNullableSchema {
    table: PgCustomNullableTest,
}

#[drizzle::test]
fn custom_postgres_column_nullable_roundtrip(db: &mut TestDb<PgCustomNullableSchema>) {
    let PgCustomNullableSchema { table } = schema;

    let value = PgU32Be(0x0a0b_0c0d);
    db.insert(table)
        .values([InsertPgCustomNullableTest::new().with_payload(value)])
        .execute();
    db.insert(table)
        .values([InsertPgCustomNullableTest::new()])
        .execute();

    let rows: Vec<SelectPgCustomNullableTest> = db
        .select((table.id, table.payload))
        .from(table)
        .order_by(table.id)
        .all();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].payload, Some(value));
    assert_eq!(rows[1].payload, None);
}

#[PostgresTable(name = "pg_custom_from_row")]
struct PgCustomFromRowTest {
    #[column(primary)]
    id: i32,
    payload: PgU32Be,
    maybe_payload: Option<PgU32Be>,
}

#[derive(PostgresSchema)]
struct PgCustomFromRowSchema {
    table: PgCustomFromRowTest,
}

#[derive(Debug, PostgresFromRow)]
struct PgCustomRow {
    payload: PgU32Be,
    maybe_payload: Option<PgU32Be>,
}

#[drizzle::test]
fn custom_postgres_column_decodes_in_from_row(db: &mut TestDb<PgCustomFromRowSchema>) {
    let PgCustomFromRowSchema { table } = schema;

    db.insert(table)
        .values([InsertPgCustomFromRowTest::new(1, PgU32Be(7)).with_maybe_payload(PgU32Be(9))])
        .execute();
    db.insert(table)
        .values([InsertPgCustomFromRowTest::new(2, PgU32Be(8))])
        .execute();

    let rows: Vec<PgCustomRow> = db
        .select(PgCustomRow::Select)
        .from(table)
        .order_by(table.id)
        .all();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].payload, PgU32Be(7));
    assert_eq!(rows[0].maybe_payload, Some(PgU32Be(9)));
    assert_eq!(rows[1].payload, PgU32Be(8));
    assert_eq!(rows[1].maybe_payload, None);
}
