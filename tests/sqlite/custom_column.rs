#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::error::DrizzleError;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::traits::DrizzleSQLiteColumn;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct U32Be(u32);

impl DrizzleSQLiteColumn for U32Be {
    type SQLType = drizzle::sqlite::types::Blob;

    const SQL_TYPE: &'static str = "BLOB";

    fn decode(value: SQLiteValueRef<'_>) -> Result<Self, DrizzleError> {
        let SQLiteValueRef::Blob(value) = value else {
            return Err(DrizzleError::ConversionError(
                "U32Be must be stored as BLOB".into(),
            ));
        };

        let bytes: [u8; 4] = value.try_into().map_err(|_| {
            DrizzleError::ConversionError(
                format!("expected 4 bytes for U32Be, got {}", value.len()).into(),
            )
        })?;
        Ok(Self(u32::from_be_bytes(bytes)))
    }

    fn encode(&self) -> SQLiteValue<'_> {
        SQLiteValue::Blob(Cow::Owned(self.0.to_be_bytes().to_vec()))
    }

    fn encode_owned(self) -> OwnedSQLiteValue {
        OwnedSQLiteValue::Blob(Box::new(self.0.to_be_bytes()))
    }
}

#[test]
fn custom_sqlite_column_decode_rejects_invalid_storage() {
    assert!(U32Be::decode(SQLiteValueRef::Text("1")).is_err());
    assert!(U32Be::decode(SQLiteValueRef::Blob(&[1, 2, 3])).is_err());
}

#[test]
fn custom_sqlite_column_converts_from_borrowed_value_ref() {
    let bytes = [0x01, 0x02, 0x03, 0x04];
    let value = SQLiteValue::Blob(Cow::Borrowed(&bytes));

    assert_eq!(value.convert::<U32Be>().unwrap(), U32Be(0x0102_0304));
}

#[SQLiteTable(NAME = "custom_u32_be_test")]
struct CustomU32BeTest {
    #[column(PRIMARY)]
    id: i32,
    payload: U32Be,
    optional_payload: Option<U32Be>,
}

#[derive(SQLiteSchema)]
struct CustomU32BeSchema {
    custom_u32_be_test: CustomU32BeTest,
}

struct CustomU32BeAlias;

impl drizzle::core::Tag for CustomU32BeAlias {
    const NAME: &'static str = "custom_u32_alias";
}

#[drizzle::test]
fn custom_sqlite_column_roundtrip_and_metadata(db: &mut TestDb<CustomU32BeSchema>) {
    let table = schema.custom_u32_be_test;

    assert!(CustomU32BeTest::ddl_sql().contains("`payload` BLOB NOT NULL"));
    assert_eq!(
        <CustomU32BeTest as drizzle::core::DrizzleTable>::TABLE_REF.columns[1].sql_type,
        "BLOB"
    );

    let value = U32Be(0x0102_0304);
    let row = InsertCustomU32BeTest::new(value).with_optional_payload(SQLiteInsertValue::Null);
    db.insert(table).values([row]).execute();

    let out: Vec<SelectCustomU32BeTest> = db
        .select((table.id, table.payload, table.optional_payload))
        .from(table)
        .r#where(and(
            and(eq(table.payload, value), eq(table.payload, &value)),
            is_null(table.optional_payload),
        ))
        .all();

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].payload, value);
    assert_eq!(out[0].optional_payload, None);

    let optional_matches: Vec<SelectCustomU32BeTest> = db
        .select((table.id, table.payload, table.optional_payload))
        .from(table)
        .r#where(eq(table.optional_payload, value))
        .all();

    assert!(optional_matches.is_empty());

    let alias = CustomU32BeTest::alias::<CustomU32BeAlias>();
    let aliased: Vec<SelectCustomU32BeTest> = db
        .select((alias.id, alias.payload, alias.optional_payload))
        .from(alias)
        .r#where(eq(alias.payload, value))
        .all();

    assert_eq!(aliased.len(), 1);
    assert_eq!(aliased[0].payload, value);

    #[derive(SQLiteFromRow, Debug)]
    struct Ty(String);
    let ty: Ty = db
        .select(r#typeof(table.payload).alias("payload_type"))
        .from(table)
        .r#where(eq(table.id, 1))
        .get();

    assert_eq!(ty.0, "blob");
}
