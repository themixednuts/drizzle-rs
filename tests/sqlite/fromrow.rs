#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

// Test struct with various data types for FromRow
#[derive(SQLiteFromRow, Debug, PartialEq)]
struct AllDataTypes {
    id: i32,
    name: String,
    age: i64,
    score: f64,
    active: bool,
    data: Vec<u8>,
}

// Test struct with different integer sizes
#[derive(SQLiteFromRow, Debug, PartialEq)]
struct IntegerTypes {
    id: i32,
    big_num: i64,
    small_num: i16,
    tiny_num: i8,
}

// Test struct with float types
#[derive(SQLiteFromRow, Debug, PartialEq)]
struct FloatTypes {
    id: i32,
    precise: f64,
    compact: f32,
}

// Test table for comprehensive type testing
#[SQLiteTable]
struct TypeTest {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[integer]
    age: i64,
    #[real]
    score: f64,
    #[boolean]
    active: bool,
    #[blob]
    data: Vec<u8>,
}

// Table for integer size testing
#[SQLiteTable]
struct IntegerTest {
    #[integer(primary)]
    id: i32,
    #[integer]
    big_num: i64,
    #[integer]
    small_num: i16,
    #[integer]
    tiny_num: i8,
}

// Table for float testing
#[SQLiteTable]
struct FloatTest {
    #[integer(primary)]
    id: i32,
    #[real]
    precise: f64,
    #[real]
    compact: f32,
}

#[derive(SQLiteSchema)]
pub struct TypeTestSchema {
    type_test: TypeTest,
}

sqlite_test!(test_fromrow_with_all_data_types, TypeTestSchema, {
    let TypeTestSchema { type_test } = schema;

    // Insert test data with all data types
    let test_data = InsertTypeTest::new("test_user", 25, 98.5, true, [1, 2, 3, 4, 5]);
    drizzle_exec!(db.insert(type_test).values([test_data]).execute());

    // Test FromRow with all data types
    let result: AllDataTypes = drizzle_exec!(db.select(()).from(type_test).get());

    let expected = AllDataTypes {
        id: 1,
        name: "test_user".to_string(),
        age: 25,
        score: 98.5,
        active: true,
        data: vec![1, 2, 3, 4, 5],
    };

    assert_eq!(result, expected);
    println!("✅ All data types test passed: {:?}", result);
});

#[derive(SQLiteSchema)]
struct IntegerSchema {
    integer_test: IntegerTest,
}

sqlite_test!(test_fromrow_with_integer_sizes, IntegerSchema, {
    let IntegerSchema { integer_test } = schema;

    // Insert test data with different integer sizes
    let test_data = InsertIntegerTest::new(9223372036854775806i64, 32000i16, 100i8);
    drizzle_exec!(db.insert(integer_test).values([test_data]).execute());

    // Test FromRow with different integer types
    let result: IntegerTypes = drizzle_exec!(db.select(()).from(integer_test).get());

    let expected = IntegerTypes {
        id: 1,
        big_num: 9223372036854775806i64,
        small_num: 32000i16,
        tiny_num: 100i8,
    };

    assert_eq!(result, expected);
    println!("✅ Integer types test passed: {:?}", result);
});

#[derive(SQLiteSchema)]
struct FloatSchema {
    float_test: FloatTest,
}
sqlite_test!(test_fromrow_with_float_types, FloatSchema, {
    let FloatSchema { float_test } = schema;

    // Insert test data with different float types
    let test_data = InsertFloatTest::new(3.141592653589793, 2.718f32);
    drizzle_exec!(db.insert(float_test).values([test_data]).execute());

    // Test FromRow with different float types
    let result: FloatTypes = drizzle_exec!(db.select(()).from(float_test).get());

    let expected = FloatTypes {
        id: 1,
        precise: 3.141592653589793,
        compact: 2.718f32,
    };

    assert_eq!(result, expected);
    println!("✅ Float types test passed: {:?}", result);
});

sqlite_test!(test_fromrow_type_conversion_edge_cases, TypeTestSchema, {
    let TypeTestSchema { type_test } = schema;

    // Insert test data with edge case values
    let test_data = InsertTypeTest::new("edge_case", 0, 0.0, false, []);
    drizzle_exec!(db.insert(type_test).values([test_data]).execute());

    // Test FromRow with edge case values
    let result: AllDataTypes = drizzle_exec!(db.select(()).from(type_test).get());

    let expected = AllDataTypes {
        id: 1,
        name: "edge_case".to_string(),
        age: 0,
        score: 0.0,
        active: false,
        data: vec![],
    };

    assert_eq!(result, expected);
    println!("✅ Edge cases test passed: {:?}", result);
});

// Test FromRow derive macro with partial selection
#[derive(SQLiteFromRow, Debug, Default)]
struct DerivedPartialSimple {
    name: String,
}

// Test FromRow with column mapping
#[derive(SQLiteFromRow, Debug, Default)]
struct DerivedSimpleWithColumns {
    #[column(TypeTest::id)]
    table_id: i32,
    #[column(TypeTest::name)]
    table_name: String,
}

sqlite_test!(
    test_fromrow_derive_with_partial_selection,
    TypeTestSchema,
    {
        let TypeTestSchema { type_test } = schema;

        let test_data = InsertTypeTest::new("derive_test", 25, 98.5, true, [1, 2, 3]);
        drizzle_exec!(db.insert(type_test).values([test_data]).execute());

        // Test the derived implementation with partial selection
        let result: DerivedPartialSimple =
            drizzle_exec!(db.select(type_test.name).from(type_test).get());
        assert_eq!(result.name, "derive_test");
    }
);

sqlite_test!(test_fromrow_with_column_mapping, TypeTestSchema, {
    let TypeTestSchema { type_test } = schema;

    let test_data = InsertTypeTest::new("column_test", 25, 98.5, true, [1, 2, 3]).with_id(42);
    drizzle_exec!(db.insert(type_test).values([test_data]).execute());

    // Test the column-mapped FromRow implementation
    let result: DerivedSimpleWithColumns = drizzle_exec!(
        db.select(DerivedSimpleWithColumns::default())
            .from(type_test)
            .get()
    );

    assert_eq!(result.table_id, 42);
    assert_eq!(result.table_name, "column_test");
});
