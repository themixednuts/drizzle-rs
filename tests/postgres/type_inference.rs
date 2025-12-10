//! Tests for PostgreSQL type inference - verifies that Rust types correctly
//! map to PostgreSQL column types and that the driver can handle them.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::OrderBy;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

// ============================================================================
// Basic Types Table - Tests core Rust primitive type inference
// ============================================================================

#[PostgresTable(name = "pg_basic_types")]
struct PgBasicTypes {
    #[column(serial, primary)]
    id: i32,
    // Integer types
    small_val: i16, // -> SMALLINT
    int_val: i32,   // -> INTEGER
    big_val: i64,   // -> BIGINT
    // Floating point types
    real_val: f32,   // -> REAL
    double_val: f64, // -> DOUBLE PRECISION
    // Boolean
    bool_val: bool, // -> BOOLEAN
    // Text types
    text_val: String, // -> TEXT
    // Binary
    blob_val: Vec<u8>, // -> BYTEA
}

#[derive(PostgresSchema)]
struct PgBasicTypesSchema {
    basic: PgBasicTypes,
}

// ============================================================================
// Optional Types Table - Tests Option<T> handling
// ============================================================================

#[PostgresTable(name = "pg_optional_types")]
struct PgOptionalTypes {
    #[column(serial, primary)]
    id: i32,
    opt_small: Option<i16>,
    opt_int: Option<i32>,
    opt_big: Option<i64>,
    opt_real: Option<f32>,
    opt_double: Option<f64>,
    opt_bool: Option<bool>,
    opt_text: Option<String>,
    opt_blob: Option<Vec<u8>>,
}

#[derive(PostgresSchema)]
struct PgOptionalTypesSchema {
    optional: PgOptionalTypes,
}

// ============================================================================
// UUID Types Table - Tests uuid::Uuid type inference
// ============================================================================

#[cfg(feature = "uuid")]
mod uuid_tests {
    use super::*;
    use uuid::Uuid;

    #[PostgresTable(name = "pg_uuid_types")]
    struct PgUuidTypes {
        #[column(primary, default_fn = Uuid::new_v4)]
        id: Uuid, // -> UUID
        opt_uuid: Option<Uuid>, // -> UUID (nullable)
    }

    #[derive(PostgresSchema)]
    struct PgUuidTypesSchema {
        uuids: PgUuidTypes,
    }

    postgres_test!(uuid_insert_and_select, PgUuidTypesSchema, {
        let PgUuidTypesSchema { uuids, .. } = schema;

        // Insert with auto-generated UUID
        let stmt = db.insert(uuids).values([InsertPgUuidTypes::new()]);
        drizzle_exec!(stmt.execute());

        // Insert with specific UUID
        let specific_id = Uuid::new_v4();
        let stmt = db
            .insert(uuids)
            .values([InsertPgUuidTypes::new().with_id(specific_id)]);
        drizzle_exec!(stmt.execute());

        // Query and verify
        let stmt = db.select(()).from(uuids);
        let results: Vec<SelectPgUuidTypes> = drizzle_exec!(stmt.all());
        assert_eq!(results.len(), 2);
        assert_eq!(results[1].id, specific_id);
    });
}

// ============================================================================
// Basic Types Tests
// ============================================================================

postgres_test!(basic_types_insert_and_select, PgBasicTypesSchema, {
    let PgBasicTypesSchema { basic, .. } = schema;

    let stmt = db.insert(basic).values([InsertPgBasicTypes::new(
        42_i16,
        12345_i32,
        9876543210_i64,
        3.14_f32,
        2.718281828_f64,
        true,
        "Hello, PostgreSQL!",
        vec![0xDE, 0xAD, 0xBE, 0xEF],
    )]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(basic);
    let results: Vec<SelectPgBasicTypes> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);

    let row = &results[0];
    assert_eq!(row.small_val, 42);
    assert_eq!(row.int_val, 12345);
    assert_eq!(row.big_val, 9876543210);
    assert!((row.real_val - 3.14).abs() < 0.001);
    assert!((row.double_val - 2.718281828).abs() < 0.00000001);
    assert!(row.bool_val);
    assert_eq!(row.text_val, "Hello, PostgreSQL!");
    assert_eq!(row.blob_val, vec![0xDE, 0xAD, 0xBE, 0xEF]);
});

postgres_test!(optional_types_with_values, PgOptionalTypesSchema, {
    let PgOptionalTypesSchema { optional, .. } = schema;

    // Insert with all values present
    let stmt = db.insert(optional).values([InsertPgOptionalTypes::new()
        .with_opt_small(100_i16)
        .with_opt_int(200)
        .with_opt_big(300_i64)
        .with_opt_real(1.5_f32)
        .with_opt_double(2.5)
        .with_opt_bool(true)
        .with_opt_text("optional text")
        .with_opt_blob(vec![1, 2, 3])]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(optional);
    let results: Vec<SelectPgOptionalTypes> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);

    let row = &results[0];
    assert_eq!(row.opt_small, Some(100));
    assert_eq!(row.opt_int, Some(200));
    assert_eq!(row.opt_big, Some(300));
    assert!(row.opt_real.is_some());
    assert!(row.opt_double.is_some());
    assert_eq!(row.opt_bool, Some(true));
    assert_eq!(row.opt_text, Some("optional text".to_string()));
    assert_eq!(row.opt_blob, Some(vec![1, 2, 3]));
});

postgres_test!(optional_types_with_nulls, PgOptionalTypesSchema, {
    let PgOptionalTypesSchema { optional, .. } = schema;

    // Insert with no optional values (all NULL)
    let stmt = db.insert(optional).values([InsertPgOptionalTypes::new()]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(optional);
    let results: Vec<SelectPgOptionalTypes> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);

    let row = &results[0];
    assert!(row.opt_small.is_none());
    assert!(row.opt_int.is_none());
    assert!(row.opt_big.is_none());
    assert!(row.opt_real.is_none());
    assert!(row.opt_double.is_none());
    assert!(row.opt_bool.is_none());
    assert!(row.opt_text.is_none());
    assert!(row.opt_blob.is_none());
});

// ============================================================================
// Integer Boundary Tests
// ============================================================================

postgres_test!(integer_boundary_values, PgBasicTypesSchema, {
    let PgBasicTypesSchema { basic, .. } = schema;

    // Test min/max values for integer types
    let stmt = db.insert(basic).values([
        InsertPgBasicTypes::new(
            i16::MIN,
            i32::MIN,
            i64::MIN,
            f32::MIN,
            f64::MIN,
            false,
            "min values",
            vec![],
        ),
        InsertPgBasicTypes::new(
            i16::MAX,
            i32::MAX,
            i64::MAX,
            f32::MAX,
            f64::MAX,
            true,
            "max values",
            vec![0xFF; 100],
        ),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(basic).order_by([OrderBy::asc(basic.id)]);
    let results: Vec<SelectPgBasicTypes> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);

    // Check min values
    assert_eq!(results[0].small_val, i16::MIN);
    assert_eq!(results[0].int_val, i32::MIN);
    assert_eq!(results[0].big_val, i64::MIN);

    // Check max values
    assert_eq!(results[1].small_val, i16::MAX);
    assert_eq!(results[1].int_val, i32::MAX);
    assert_eq!(results[1].big_val, i64::MAX);
});

// ============================================================================
// Text and Binary Edge Cases
// ============================================================================

postgres_test!(text_special_characters, PgBasicTypesSchema, {
    let PgBasicTypesSchema { basic, .. } = schema;

    let special_text = "Hello! こんにちは 🦀 'quotes' \"double\" \n\t\\backslash";

    let stmt = db.insert(basic).values([InsertPgBasicTypes::new(
        1_i16,
        1,
        1_i64,
        1.0_f32,
        1.0,
        true,
        special_text,
        vec![],
    )]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(basic);
    let results: Vec<SelectPgBasicTypes> = drizzle_exec!(stmt.all());
    assert_eq!(results[0].text_val, special_text);
});

postgres_test!(binary_data_roundtrip, PgBasicTypesSchema, {
    let PgBasicTypesSchema { basic, .. } = schema;

    // Test various binary patterns
    let binary_data: Vec<u8> = (0..=255).collect();

    let stmt = db.insert(basic).values([InsertPgBasicTypes::new(
        1_i16,
        1,
        1_i64,
        1.0_f32,
        1.0,
        true,
        "binary test",
        binary_data.clone(),
    )]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(basic);
    let results: Vec<SelectPgBasicTypes> = drizzle_exec!(stmt.all());
    assert_eq!(results[0].blob_val, binary_data);
});

// ============================================================================
// Chrono Types Tests (feature-gated)
// ============================================================================

#[cfg(feature = "chrono")]
mod chrono_tests {
    use super::*;
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    #[PostgresTable(name = "pg_chrono_types")]
    struct PgChronoTypes {
        #[column(serial, primary)]
        id: i32,
        date_val: NaiveDate,            // -> DATE
        time_val: NaiveTime,            // -> TIME
        timestamp_val: NaiveDateTime,   // -> TIMESTAMP
        timestamptz_val: DateTime<Utc>, // -> TIMESTAMPTZ
    }

    #[derive(PostgresSchema)]
    struct PgChronoTypesSchema {
        chrono: PgChronoTypes,
    }

    postgres_test!(chrono_types_roundtrip, PgChronoTypesSchema, {
        let PgChronoTypesSchema { chrono, .. } = schema;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let time = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
        let timestamp = NaiveDateTime::new(date, time);
        let timestamptz = DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc);

        let stmt = db.insert(chrono).values([InsertPgChronoTypes::new(
            date,
            time,
            timestamp,
            timestamptz,
        )]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(chrono);
        let results: Vec<SelectPgChronoTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].date_val, date);
        assert_eq!(results[0].time_val, time);
        assert_eq!(results[0].timestamp_val, timestamp);
        // Note: timestamptz comparison may need timezone handling
    });
}

// ============================================================================
// Geo Types Tests (feature-gated)
// ============================================================================

#[cfg(feature = "geo-types")]
mod geo_tests {
    use super::*;
    use geo_types::Point;

    #[PostgresTable(name = "pg_geo_types")]
    struct PgGeoTypes {
        #[column(serial, primary)]
        id: i32,
        point_val: Point<f64>, // -> POINT
    }

    #[derive(PostgresSchema)]
    struct PgGeoTypesSchema {
        geo: PgGeoTypes,
    }

    postgres_test!(geo_point_roundtrip, PgGeoTypesSchema, {
        let PgGeoTypesSchema { geo, .. } = schema;

        let point = Point::new(40.7128, -74.0060); // NYC coordinates

        let stmt = db.insert(geo).values([InsertPgGeoTypes::new(point)]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(geo);
        let results: Vec<SelectPgGeoTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].point_val, point);
    });
}

// ============================================================================
// Network Types Tests (feature-gated)
// ============================================================================

#[cfg(feature = "cidr")]
mod cidr_tests {
    use super::*;
    use cidr::{IpCidr, IpInet};
    use std::str::FromStr;

    // Note: cidr types don't implement Default, so we use Option<T> fields
    #[PostgresTable(name = "pg_network_types")]
    struct PgNetworkTypes {
        #[column(serial, primary)]
        id: i32,
        inet_val: Option<IpInet>, // -> INET (nullable)
        cidr_val: Option<IpCidr>, // -> CIDR (nullable)
    }

    #[derive(PostgresSchema)]
    struct PgNetworkTypesSchema {
        network: PgNetworkTypes,
    }

    postgres_test!(network_types_roundtrip, PgNetworkTypesSchema, {
        let PgNetworkTypesSchema { network, .. } = schema;

        let inet = IpInet::from_str("192.168.1.100/24").unwrap();
        let cidr = IpCidr::from_str("10.0.0.0/8").unwrap();

        let stmt = db.insert(network).values([InsertPgNetworkTypes::new()
            .with_inet_val(inet)
            .with_cidr_val(cidr)]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(network);
        let results: Vec<SelectPgNetworkTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].inet_val, Some(inet));
        assert_eq!(results[0].cidr_val, Some(cidr));
    });
}

// ============================================================================
// Bit Vector Tests (feature-gated)
// ============================================================================

#[cfg(feature = "bit-vec")]
mod bitvec_tests {
    use super::*;
    use bit_vec::BitVec;

    #[PostgresTable(name = "pg_bitvec_types")]
    struct PgBitVecTypes {
        #[column(serial, primary)]
        id: i32,
        bits: BitVec, // -> BIT VARYING
    }

    #[derive(PostgresSchema)]
    struct PgBitVecTypesSchema {
        bitvec: PgBitVecTypes,
    }

    postgres_test!(bitvec_roundtrip, PgBitVecTypesSchema, {
        let PgBitVecTypesSchema { bitvec, .. } = schema;

        let mut bits = BitVec::new();
        bits.push(true);
        bits.push(false);
        bits.push(true);
        bits.push(true);
        bits.push(false);
        bits.push(true);
        bits.push(false);
        bits.push(false);

        let stmt = db
            .insert(bitvec)
            .values([InsertPgBitVecTypes::new(bits.clone())]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(bitvec);
        let results: Vec<SelectPgBitVecTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].bits, bits);
    });
}

// ============================================================================
// ArrayVec/ArrayString Types Tests (feature-gated)
// ============================================================================

#[cfg(feature = "arrayvec")]
mod arrayvec_tests {
    use super::*;
    use arrayvec::{ArrayString, ArrayVec};

    #[PostgresTable(name = "pg_arrayvec_types")]
    struct PgArrayVecTypes {
        #[column(serial, primary)]
        id: i32,
        fixed_string: ArrayString<32>, // -> VARCHAR(32)
        fixed_blob: ArrayVec<u8, 64>,  // -> BYTEA
    }

    #[derive(PostgresSchema)]
    struct PgArrayVecTypesSchema {
        arrayvec: PgArrayVecTypes,
    }

    postgres_test!(arrayvec_roundtrip, PgArrayVecTypesSchema, {
        let PgArrayVecTypesSchema { arrayvec, .. } = schema;

        let fixed_string = ArrayString::try_from("Hello, ArrayString!").unwrap();
        let mut fixed_blob: ArrayVec<u8, 64> = ArrayVec::new();
        fixed_blob.extend([0xDE, 0xAD, 0xBE, 0xEF]);

        let stmt = db
            .insert(arrayvec)
            .values([InsertPgArrayVecTypes::new(fixed_string, fixed_blob.clone())]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(arrayvec);
        let results: Vec<SelectPgArrayVecTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fixed_string, fixed_string);
        assert_eq!(results[0].fixed_blob, fixed_blob);
    });
}

// ============================================================================
// serde_json Types Tests (feature-gated)
// ============================================================================

#[cfg(feature = "serde")]
mod json_tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    // Custom struct for JSON storage
    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
    struct Metadata {
        theme: String,
        notifications: bool,
        settings: Vec<String>,
    }

    // Another custom struct for testing nested types
    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
    struct UserProfile {
        name: String,
        age: u32,
        preferences: Metadata,
    }

    // Test explicit JSON column type with custom struct
    #[PostgresTable(name = "pg_json_types")]
    struct PgJsonTypes {
        #[column(serial, primary)]
        id: i32,
        #[column(json)]
        metadata: Metadata, // -> JSON with custom struct
        #[column(json)]
        opt_metadata: Option<Metadata>, // -> JSON (nullable) with custom struct
        #[column(json)]
        raw_json: serde_json::Value, // -> JSON with raw Value
    }

    // Test explicit JSONB column type with custom struct
    #[PostgresTable(name = "pg_jsonb_types")]
    struct PgJsonbTypes {
        #[column(serial, primary)]
        id: i32,
        #[column(jsonb)]
        profile: UserProfile, // -> JSONB with nested custom struct
        #[column(jsonb)]
        opt_profile: Option<UserProfile>, // -> JSONB (nullable) with custom struct
        #[column(jsonb)]
        raw_jsonb: serde_json::Value, // -> JSONB with raw Value
    }

    #[derive(PostgresSchema)]
    struct PgJsonTypesSchema {
        json_table: PgJsonTypes,
        jsonb_table: PgJsonbTypes,
    }

    postgres_test!(json_custom_struct_roundtrip, PgJsonTypesSchema, {
        let PgJsonTypesSchema { json_table, .. } = schema;

        let metadata = Metadata {
            theme: "dark".to_string(),
            notifications: true,
            settings: vec!["setting1".to_string(), "setting2".to_string()],
        };
        let raw_json = json!({"extra": "data"});

        let stmt = db
            .insert(json_table)
            .values([InsertPgJsonTypes::new(metadata.clone(), raw_json.clone())]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(json_table);
        let results: Vec<SelectPgJsonTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata, metadata);
        assert_eq!(results[0].raw_json, raw_json);
        assert_eq!(results[0].opt_metadata, None);
    });

    postgres_test!(jsonb_nested_struct_roundtrip, PgJsonTypesSchema, {
        let PgJsonTypesSchema { jsonb_table, .. } = schema;

        let profile = UserProfile {
            name: "Alice".to_string(),
            age: 30,
            preferences: Metadata {
                theme: "light".to_string(),
                notifications: false,
                settings: vec!["compact".to_string()],
            },
        };
        let raw_jsonb = json!({"key": "value"});

        let stmt = db
            .insert(jsonb_table)
            .values([InsertPgJsonbTypes::new(profile.clone(), raw_jsonb.clone())]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(jsonb_table);
        let results: Vec<SelectPgJsonbTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].profile, profile);
        assert_eq!(results[0].raw_jsonb, raw_jsonb);
        assert_eq!(results[0].opt_profile, None);
    });

    postgres_test!(json_with_optional_struct, PgJsonTypesSchema, {
        let PgJsonTypesSchema { json_table, .. } = schema;

        let metadata = Metadata {
            theme: "system".to_string(),
            notifications: true,
            settings: vec![],
        };
        let opt_metadata = Metadata {
            theme: "custom".to_string(),
            notifications: false,
            settings: vec!["advanced".to_string()],
        };
        let raw_json = json!({});

        let stmt = db.insert(json_table).values([InsertPgJsonTypes::new(
            metadata.clone(),
            raw_json.clone(),
        )
        .with_opt_metadata(opt_metadata.clone())]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(json_table);
        let results: Vec<SelectPgJsonTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata, metadata);
        assert_eq!(results[0].opt_metadata, Some(opt_metadata));
    });

    postgres_test!(jsonb_with_optional_struct, PgJsonTypesSchema, {
        let PgJsonTypesSchema { jsonb_table, .. } = schema;

        let profile = UserProfile {
            name: "Bob".to_string(),
            age: 25,
            preferences: Metadata::default(),
        };
        let opt_profile = UserProfile {
            name: "Alice".to_string(),
            age: 28,
            preferences: Metadata {
                theme: "dark".to_string(),
                notifications: true,
                settings: vec!["beta".to_string()],
            },
        };
        let raw_jsonb = json!(null);

        let stmt = db.insert(jsonb_table).values([InsertPgJsonbTypes::new(
            profile.clone(),
            raw_jsonb.clone(),
        )
        .with_opt_profile(opt_profile.clone())]);
        drizzle_exec!(stmt.execute());

        let stmt = db.select(()).from(jsonb_table);
        let results: Vec<SelectPgJsonbTypes> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].profile, profile);
        assert_eq!(results[0].opt_profile, Some(opt_profile));
    });
}
