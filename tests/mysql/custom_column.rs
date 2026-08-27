#![cfg(any(feature = "mysql-sync", feature = "mysql-async"))]

use drizzle::core::expr::{and, eq};
use drizzle::error::DrizzleError;
use drizzle::mysql::prelude::*;
use drizzle::mysql::traits::DrizzleMySQLColumn;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct U32Be(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct SignedValue(i64);

impl DrizzleMySQLColumn for U32Be {
    type SQLType = drizzle::mysql::types::Binary;

    const SQL_TYPE: &'static str = "BINARY(4)";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let MySQLValue::Bytes(value) = value else {
            return Err(DrizzleError::ConversionError(
                "U32Be must be stored as binary data".into(),
            ));
        };
        let bytes: [u8; 4] = value.as_ref().try_into().map_err(|_| {
            DrizzleError::ConversionError(
                format!("expected 4 bytes for U32Be, got {}", value.len()).into(),
            )
        })?;
        Ok(Self(u32::from_be_bytes(bytes)))
    }

    fn encode(&self) -> MySQLValue<'_> {
        MySQLValue::Bytes(Cow::Owned(self.0.to_be_bytes().to_vec()))
    }

    fn encode_owned(self) -> OwnedMySQLValue {
        OwnedMySQLValue::Bytes(self.0.to_be_bytes().to_vec())
    }
}

impl DrizzleMySQLColumn for SignedValue {
    type SQLType = drizzle::mysql::types::BigInt;

    const SQL_TYPE: &'static str = "BIGINT";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let MySQLValue::Int(value) = value else {
            return Err(DrizzleError::ConversionError(
                "SignedValue must use the signed integer wire representation".into(),
            ));
        };
        Ok(Self(value))
    }

    fn encode(&self) -> MySQLValue<'_> {
        MySQLValue::Int(self.0)
    }
}

#[MySQLTable(NAME = "mysql_custom_u32_be_test")]
struct CustomU32BeTest {
    #[column(PRIMARY)]
    id: i32,
    payload: U32Be,
    signed: SignedValue,
    optional_payload: Option<U32Be>,
}

#[derive(MySQLSchema)]
struct CustomU32BeSchema {
    values: CustomU32BeTest,
    payload_idx: CustomU32BePayloadIdx,
}

#[MySQLIndex]
struct CustomU32BePayloadIdx(CustomU32BeTest::payload);

struct CustomU32BeAlias;

impl drizzle::core::Tag for CustomU32BeAlias {
    const NAME: &'static str = "mysql_custom_u32_alias";
}

#[test]
fn custom_mysql_column_rejects_invalid_storage() {
    assert!(U32Be::decode(MySQLValue::Int(1)).is_err());
    assert!(U32Be::decode(MySQLValue::from(vec![1, 2, 3])).is_err());
}

#[test]
fn custom_mysql_column_owns_encoding_and_metadata() {
    let value = U32Be(0x0102_0304);
    assert_eq!(value.encode().as_bytes(), Some([1, 2, 3, 4].as_slice()));
    assert!(CustomU32BeTest::ddl_sql().contains("`payload` BINARY(4) NOT NULL"));
    assert_eq!(
        <CustomU32BeTest as drizzle::core::DrizzleTable>::TABLE_REF.columns[1].sql_type,
        "BINARY(4)"
    );
    assert!(CustomU32BeTest::ddl_sql().contains("`signed` BIGINT NOT NULL"));
    assert!(CustomU32BePayloadIdx::DDL_SQL.contains("(`payload`)"));
}

#[cfg(feature = "query")]
#[test]
fn projected_custom_values_match_wire_variants() {
    use drizzle::core::query::{JsonProjectionKind, QueryTable};

    let binary = drizzle::core::serde_json::json!({
        "$drizzle_storage": "blob",
        "$drizzle_value": "01020304",
    });
    let signed = drizzle::core::serde_json::Value::String((i64::MIN + 7).to_string());

    assert_eq!(
        drizzle::mysql::driver::decode_projected::<U32Be>(&binary).unwrap(),
        U32Be(0x0102_0304)
    );
    assert_eq!(
        drizzle::mysql::driver::decode_projected::<SignedValue>(&signed).unwrap(),
        SignedValue(i64::MIN + 7)
    );
    assert_eq!(
        CustomU32BeTest::JSON_PROJECTIONS
            .iter()
            .find(|projection| projection.column == "payload")
            .unwrap()
            .kind,
        JsonProjectionKind::TaggedHex
    );
    assert_eq!(
        CustomU32BeTest::JSON_PROJECTIONS
            .iter()
            .find(|projection| projection.column == "signed")
            .unwrap()
            .kind,
        JsonProjectionKind::Text
    );
}

#[drizzle::test]
fn custom_mysql_column_round_trips_through_both_adapters(db: &mut TestDb<CustomU32BeSchema>) {
    let CustomU32BeSchema { values, .. } = schema;
    let value = U32Be(0x0102_0304);
    let signed = SignedValue(i64::MIN + 7);
    db.insert(values)
        .value(InsertCustomU32BeTest::new(1, value, signed).with_optional_payload(value))
        .execute();

    let selected: SelectCustomU32BeTest = db
        .select(())
        .from(values)
        .r#where(and(eq(values.payload, value), eq(values.payload, &value)))
        .get();
    assert_eq!(selected.payload, value);
    assert_eq!(selected.signed, signed);
    assert_eq!(selected.optional_payload, Some(value));

    let replacement = U32Be(0x0a0b_0c0d);
    db.update(values)
        .set(UpdateCustomU32BeTest::default().with_payload(replacement))
        .r#where(eq(values.id, 1))
        .execute();

    let alias = CustomU32BeTest::alias::<CustomU32BeAlias>();
    let selected: SelectCustomU32BeTest = db
        .select(())
        .from(alias)
        .r#where(eq(alias.payload, &replacement))
        .get();
    assert_eq!(selected.payload, replacement);
}

#[cfg(feature = "query")]
#[drizzle::test(mysql)]
fn custom_mysql_column_uses_its_codec_in_relational_queries(db: &mut TestDb<CustomU32BeSchema>) {
    let CustomU32BeSchema { values, .. } = schema;
    let value = U32Be(0x1020_3040);
    let signed = SignedValue(i64::MIN + 9);
    db.insert(values)
        .value(InsertCustomU32BeTest::new(1, value, signed))
        .execute();

    let selected = db.query(values).find_first().unwrap();
    assert_eq!(selected.payload, value);
    assert_eq!(selected.signed, signed);
    assert_eq!(selected.optional_payload, None);
}
