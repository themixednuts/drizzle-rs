use drizzle::core::expr::CastTarget;
use drizzle::core::types as core_types;
use drizzle::postgres::prelude::*;
use drizzle::postgres::types as postgres_types;
use drizzle_core::dialect::PostgresDialect;

#[PostgresTable]
struct BoundedCharacters {
    #[column(VARCHAR(255))]
    varchar_value: String,
    #[column(CHAR(8))]
    char_value: String,
    #[column(VARCHAR(64))]
    aliases: Vec<String>,
}

#[PostgresView(EXISTING)]
struct BoundedCharacterView {
    #[column(VARCHAR(32))]
    label: String,
}

#[test]
fn explicit_bounded_characters_preserve_physical_types() {
    let sql = BoundedCharacters::create_table_sql();
    assert!(sql.contains("\"varchar_value\" VARCHAR(255) NOT NULL"));
    assert!(sql.contains("\"char_value\" CHAR(8) NOT NULL"));
    assert!(sql.contains("\"aliases\" VARCHAR(64)[] NOT NULL"));
    assert_eq!(
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[0].sql_type,
        "VARCHAR(255)"
    );
    assert_eq!(
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[1].sql_type,
        "CHAR(8)"
    );
    assert_eq!(
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[2].sql_type,
        "VARCHAR(64)"
    );
    let drizzle::core::ColumnDialect::PostgreSQL { dimensions, .. } =
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[2].dialect
    else {
        panic!("expected PostgreSQL column metadata");
    };
    assert_eq!(dimensions, Some(1));
    assert_eq!(
        <BoundedCharacterView as drizzle::core::DrizzleTable>::TABLE_REF.columns[0].sql_type,
        "VARCHAR(32)"
    );
}

#[test]
fn postgres_dialect_types_are_distinct_markers_with_cast_mappings() {
    fn assert_target<T: core_types::DataType, C: CastTarget<'static, T, PostgresDialect>>(_: C) {}

    let _ = postgres_types::Int2;
    let _ = postgres_types::Int4;
    let _ = postgres_types::Int8;
    let _ = postgres_types::Float4;
    let _ = postgres_types::Float8;
    let _ = postgres_types::Varchar;
    let _ = postgres_types::Bytea;
    let _ = postgres_types::Boolean;
    let _ = postgres_types::Timestamptz;

    assert_target::<postgres_types::Int2, _>(postgres_types::Int2);
    assert_target::<postgres_types::Int4, _>(postgres_types::Int4);
    assert_target::<postgres_types::Int8, _>(postgres_types::Int8);
    assert_target::<postgres_types::Float4, _>(postgres_types::Float4);
    assert_target::<postgres_types::Float8, _>(postgres_types::Float8);
    assert_target::<postgres_types::Varchar, _>(postgres_types::Varchar);
    assert_target::<postgres_types::Bytea, _>(postgres_types::Bytea);
    assert_target::<postgres_types::Boolean, _>(postgres_types::Boolean);
    assert_target::<postgres_types::Timestamptz, _>(postgres_types::Timestamptz);

    fn assert_compatible<S, T>()
    where
        S: core_types::DataType + core_types::Compatible<T>,
        T: core_types::DataType,
    {
    }

    // Postgres numeric widening
    assert_compatible::<postgres_types::Int2, postgres_types::Int4>();
    assert_compatible::<postgres_types::Int4, postgres_types::Int8>();
    assert_compatible::<postgres_types::Float4, postgres_types::Float8>();
    // Cross int/float
    assert_compatible::<postgres_types::Int4, postgres_types::Float8>();
    // Numeric compat
    assert_compatible::<postgres_types::Numeric, postgres_types::Int4>();
    // Text variants
    assert_compatible::<postgres_types::Text, postgres_types::Varchar>();
    // Temporal
    assert_compatible::<postgres_types::Timestamp, postgres_types::Timestamptz>();
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
mod fixed_size_bytes {
    use drizzle::core::expr::eq;
    use drizzle::postgres::prelude::*;

    #[PostgresTable(name = "fixed_size_bytes")]
    struct FixedSizeBytes {
        #[column(primary)]
        id: i32,
        digest: [u8; 4],
        maybe_digest: Option<[u8; 4]>,
    }

    #[derive(PostgresSchema)]
    struct FixedSizeBytesSchema {
        table: FixedSizeBytes,
    }

    #[derive(Debug, PostgresFromRow)]
    struct DigestRow {
        digest: [u8; 4],
        maybe_digest: Option<[u8; 4]>,
    }

    #[drizzle::test]
    fn fixed_size_byte_arrays_round_trip(db: &mut TestDb<FixedSizeBytesSchema>) {
        let FixedSizeBytesSchema { table } = schema;

        db.insert(table)
            .values([InsertFixedSizeBytes::new(1, [1, 2, 3, 4]).with_maybe_digest([9, 9, 9, 9])])
            .execute();
        db.insert(table)
            .values([InsertFixedSizeBytes::new(2, [5, 6, 7, 8])])
            .execute();

        let rows: Vec<SelectFixedSizeBytes> = db
            .select(())
            .from(table)
            .r#where(eq(table.digest, [1, 2, 3, 4]))
            .all();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].digest, [1, 2, 3, 4]);
        assert_eq!(rows[0].maybe_digest, Some([9, 9, 9, 9]));

        let digests: Vec<([u8; 4], Option<[u8; 4]>)> = db
            .select((table.digest, table.maybe_digest))
            .from(table)
            .order_by(table.id)
            .all();
        assert_eq!(
            digests,
            [([1, 2, 3, 4], Some([9, 9, 9, 9])), ([5, 6, 7, 8], None)]
        );

        let from_row: Vec<DigestRow> = db
            .select(DigestRow::Select)
            .from(table)
            .order_by(table.id)
            .all();
        assert_eq!(from_row[1].digest, [5, 6, 7, 8]);
        assert_eq!(from_row[1].maybe_digest, None);

        db.update(table)
            .set(UpdateFixedSizeBytes::default().with_digest([0, 0, 0, 1]))
            .r#where(eq(table.id, 2))
            .execute();
        let updated: SelectFixedSizeBytes =
            db.select(()).from(table).r#where(eq(table.id, 2)).get();
        assert_eq!(updated.digest, [0, 0, 0, 1]);
    }
}
