#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::InsertComplex;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};
#[cfg(feature = "serde")]
use crate::common::schema::sqlite::{UserConfig, UserMetadata};

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;

#[cfg(feature = "uuid")]
#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

#[drizzle::test]
fn simple_select_with_conditions(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Insert test data
    let test_data = vec![
        InsertSimple::new("alpha"),
        InsertSimple::new("beta"),
        InsertSimple::new("gamma"),
        InsertSimple::new("delta"),
    ];

    let stmt = db.insert(simple).values(test_data);
    stmt.execute();

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "beta"));

    // Test WHERE condition
    let where_results: Vec<SelectSimple> = stmt.all();

    assert_eq!(where_results.len(), 1);
    assert_eq!(where_results[0].name, "beta");

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(2);
    // Test ORDER BY with LIMIT
    let ordered_results: Vec<SelectSimple> = stmt.all();

    assert_eq!(ordered_results.len(), 2);
    assert_eq!(ordered_results[0].name, "alpha");
    assert_eq!(ordered_results[1].name, "beta");

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(2)
        .offset(2);

    // Test LIMIT with OFFSET
    let offset_results: Vec<SelectSimple> = stmt.all();

    assert_eq!(offset_results.len(), 2);
    assert_eq!(offset_results[0].name, "delta");
    assert_eq!(offset_results[1].name, "gamma");
}

#[drizzle::test]
fn select_limit_offset_with_placeholders(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([
            InsertSimple::new("alpha").with_id(1),
            InsertSimple::new("beta").with_id(2),
            InsertSimple::new("gamma").with_id(3),
            InsertSimple::new("delta").with_id(4),
        ])
        .execute();

    let literal = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(2)
        .offset(1);
    assert_eq!(
        literal.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" ORDER BY "simple"."name" ASC LIMIT 2 OFFSET 1"#
    );

    let untyped_limit = drizzle::core::Placeholder::named("limit");
    let untyped_offset = drizzle::core::Placeholder::named("offset");
    let untyped_stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(untyped_limit)
        .offset(untyped_offset);
    assert_eq!(
        untyped_stmt.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" ORDER BY "simple"."name" ASC LIMIT :limit OFFSET :offset"#
    );

    let limit = simple.id.placeholder("limit");
    let offset = simple.id.placeholder("offset");
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(limit)
        .offset(offset)
        .prepare()
        .into_owned();

    let rows: Vec<SelectSimple> = prepared.all(db.conn(), [limit.bind(2), offset.bind(1)]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].name, "beta");
    assert_eq!(rows[1].name, "delta");
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn complex_select_with_conditions(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex } = schema;
    // Insert test data with different ages
    #[cfg(not(feature = "uuid"))]
    let test_data = [
        InsertComplex::new("young", true, Role::User)
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::new("middle", true, Role::User)
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::new("old", true, Role::User)
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    #[cfg(feature = "uuid")]
    let test_data = [
        InsertComplex::new("young", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::new("middle", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::new("old", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    let stmt = db.insert(complex).values(test_data);
    stmt.execute();

    // Test complex WHERE with GT condition
    let gt_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(gt(complex.age, 25))
        .all();

    assert_eq!(gt_results.len(), 2);
    let names: Vec<String> = gt_results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"middle".to_string()));
    assert!(names.contains(&"old".to_string()));

    // Test complex WHERE with range conditions (AND logic)
    let range_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(and(gte(complex.age, 25), lt(complex.age, 45)))
        .all();

    assert_eq!(range_results.len(), 1);
    assert_eq!(range_results[0].name, "middle");
    assert_eq!(range_results[0].age, Some(35));
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[drizzle::test]
fn feature_gated_select(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex } = schema;

    // Insert Complex record with feature-gated fields
    let test_id = uuid::Uuid::new_v4();
    let data = InsertComplex::new("feature_user", true, Role::User)
        .with_id(test_id)
        .with_metadata(UserMetadata {
            preferences: vec!["admin_panel".to_string()],
            last_login: Some("2023-12-01".to_string()),
            theme: "admin".to_string(),
        })
        .with_config(UserConfig {
            notifications: false,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    db.insert(complex).values([data]).execute();

    // Query using UUID field
    let uuid_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(eq(complex.id, test_id))
        .all();

    assert_eq!(uuid_results.len(), 1);
    assert_eq!(uuid_results[0].name, "feature_user");

    // Query using name to verify metadata exists (can't easily verify content without custom result type)
    let metadata_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(eq(complex.name, "feature_user"))
        .all();

    assert_eq!(metadata_results.len(), 1);
    assert_eq!(metadata_results[0].name, "feature_user");
}

// NUMERIC affinity stores a whole-number REAL as INTEGER, so these cells come
// back from SQLite with a different storage class than the field's type.
#[SQLiteTable(NAME = "numeric_affinity")]
struct NumericAffinity {
    #[column(primary)]
    id: i32,
    #[column(numeric)]
    price: f64,
    #[column(numeric)]
    discount: Option<f64>,
}

#[derive(SQLiteSchema)]
struct NumericAffinitySchema {
    numeric_affinity: NumericAffinity,
}

#[drizzle::test]
fn numeric_affinity_reals_stored_as_integers_decode(db: &mut TestDb<NumericAffinitySchema>) {
    let NumericAffinitySchema { numeric_affinity } = schema;

    db.insert(numeric_affinity)
        .values([InsertNumericAffinity::new(10.0).with_discount(4.0)])
        .execute();

    let rows: Vec<SelectNumericAffinity> = db.select(()).from(numeric_affinity).all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].price, 10.0);
    assert_eq!(rows[0].discount, Some(4.0));
}

#[SQLiteTable(NAME = "loose_integers")]
struct LooseInteger {
    #[column(primary)]
    id: i32,
    n: i64,
    small: i32,
    flag: bool,
}

#[derive(SQLiteSchema)]
struct LooseIntegerSchema {
    loose_integers: LooseInteger,
}

/// A column without INTEGER affinity keeps integers written as TEXT or REAL
/// in that storage class. A selected column decodes them as a table model
/// does: integer TEXT, an integral REAL in range, and `'1'`/`'yes'` or
/// `1.0` for a bool.
#[drizzle::test]
fn integers_stored_as_text_or_real_decode_in_selected_columns(db: &mut TestDb<LooseIntegerSchema>) {
    let LooseIntegerSchema { loose_integers } = schema;
    result!(db.execute(SQL::raw(r#"DROP TABLE "loose_integers""#)))?;
    result!(db.execute(SQL::raw(
        r#"CREATE TABLE "loose_integers" ("id" INTEGER PRIMARY KEY, "n", "small", "flag")"#
    )))?;
    result!(db.execute(SQL::raw(
        r#"INSERT INTO "loose_integers" VALUES
           (1, '42', '-7', '1'),
           (2, 42.0, -7.0, 1.0),
           (3, '9007199254740993', '0', 'no')"#
    )))?;

    let expected = [
        (42, -7, true),
        (42, -7, true),
        (9_007_199_254_740_993, 0, false),
    ];
    let selected: Vec<(i64, i32, bool)> = db
        .select((loose_integers.n, loose_integers.small, loose_integers.flag))
        .from(loose_integers)
        .order_by([asc(loose_integers.id)])
        .all();
    assert_eq!(selected, expected);

    let models: Vec<SelectLooseInteger> = db
        .select(())
        .from(loose_integers)
        .order_by([asc(loose_integers.id)])
        .all();
    let from_models: Vec<_> = models
        .iter()
        .map(|row| (row.n, row.small, row.flag))
        .collect();
    assert_eq!(from_models, expected);

    // Values that are not integers fail in both, rather than truncating.
    result!(db.execute(SQL::raw(
        r#"UPDATE "loose_integers" SET "n" = 1.5, "small" = '3000000000', "flag" = 'maybe'"#
    )))?;
    let n: Result<Vec<(i64,)>, _> =
        result!(db.select((loose_integers.n,)).from(loose_integers).all());
    assert!(n.is_err(), "a fractional REAL is not an i64");
    let small: Result<Vec<(i32,)>, _> = result!(
        db.select((loose_integers.small,))
            .from(loose_integers)
            .all()
    );
    assert!(small.is_err(), "TEXT out of range for i32");
    let flag: Result<Vec<(bool,)>, _> =
        result!(db.select((loose_integers.flag,)).from(loose_integers).all());
    assert!(flag.is_err(), "TEXT that is not a bool");
    let models: Result<Vec<SelectLooseInteger>, _> =
        result!(db.select(()).from(loose_integers).all());
    assert!(models.is_err());
}

/// ORDER BY after a set operation keeps `get` an `Option`: the set
/// operation can remove an aggregate's one row.
#[drizzle::test]
fn get_after_a_set_operation_and_order_by_is_an_option(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    db.insert(simple).values([InsertSimple::new("a")]).execute();

    let qb = || drizzle::sqlite::builder::QueryBuilder::new::<SimpleSchema>();
    let removed: Option<i64> = db
        .select(count(simple.id))
        .from(simple)
        .except(qb().select(count(simple.id)).from(simple))
        .order_by(SQL::raw("1"))
        .get();
    assert_eq!(removed, None);
}

/// No matching row is `None` on every driver, from the builder's `get` and
/// from `get` on raw SQL alike.
#[drizzle::test]
fn get_without_a_matching_row_is_none(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let missing: Option<SelectSimple> =
        db.select(()).from(simple).r#where(eq(simple.id, 999)).get();
    assert_eq!(missing, None);

    let raw: Option<SelectSimple> = db.get(SQL::raw("SELECT id, name FROM simple WHERE id = 999"));
    assert_eq!(raw, None);
}

#[drizzle::test]
fn count_runs_and_nests_as_a_subquery(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([
            InsertSimple::new("alpha").with_id(1),
            InsertSimple::new("beta").with_id(2),
            InsertSimple::new("gamma").with_id(3),
        ])
        .execute();

    let all = db.count(simple);
    assert_eq!(all.to_sql().sql(), r#"SELECT COUNT(*) FROM "simple""#);
    let total: i64 = all.get();
    assert_eq!(total, 3);

    let filtered = db.count(simple).r#where(gt(simple.id, 1));
    assert_eq!(
        filtered.to_sql().sql(),
        r#"SELECT COUNT(*) FROM "simple" WHERE "simple"."id" > ?"#
    );
    let above_one: i64 = filtered.get();
    assert_eq!(above_one, 2);

    let none: i64 = db.count(simple).r#where(gt(simple.id, 10)).get();
    assert_eq!(none, 0);

    let below_total = db
        .select(simple.name)
        .from(simple)
        .r#where(lt(simple.id, db.count(simple)));
    assert_eq!(
        below_total.to_sql().sql(),
        r#"SELECT "simple"."name" FROM "simple" WHERE "simple"."id" <(SELECT COUNT(*) FROM "simple")"#
    );
    let names: Vec<String> = below_total.all();
    assert_eq!(names, ["alpha", "beta"]);

    let in_tx: Result<i64, _> = result!(db.transaction(
        drizzle::sqlite::connection::SQLiteTransactionType::Deferred,
        |tx| result!(tx.count(simple).r#where(lt(simple.id, 3)).get())
    ));
    assert_eq!(in_tx.expect("count inside a transaction"), 2);
}
