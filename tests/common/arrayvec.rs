/// Round trips for `arrayvec::ArrayString` (bounded text) and
/// `arrayvec::ArrayVec<u8, N>` (bounded binary) columns.
///
/// Instantiate under `#[cfg(feature = "arrayvec")]`. SQLite additionally
/// checks the storage class these types land in (see
/// `tests/sqlite/arrayvec.rs`).
macro_rules! shared_arrayvec_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_arrayvec {
            use super::*;
            use arrayvec::{ArrayString, ArrayVec};
            use drizzle::core::expr::eq;

            #[$table(NAME = "shared_arrayvec_rows")]
            struct ArrayRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                /// Bounded text, inferred from the capacity.
                name: ArrayString<16>,
                /// Bounded binary, inferred from the capacity.
                data: ArrayVec<u8, 32>,
                label: String,
            }

            #[$table(NAME = "shared_arrayvec_mixed")]
            struct MixedRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                short_name: ArrayString<8>,
                long_name: ArrayString<64>,
                small_data: ArrayVec<u8, 16>,
                large_data: ArrayVec<u8, 128>,
            }

            #[$table(NAME = "shared_arrayvec_nullable")]
            struct NullableRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: Option<ArrayString<16>>,
                data: Option<ArrayVec<u8, 32>>,
            }

            #[derive($schema)]
            struct SharedArrayVecSchema {
                rows: ArrayRow,
                mixed: MixedRow,
                nullable: NullableRow,
            }

            fn bytes<const N: usize>(values: impl IntoIterator<Item = u8>) -> ArrayVec<u8, N> {
                let mut data = ArrayVec::new();
                data.extend(values);
                data
            }

            #[drizzle::test($dialect)]
            fn bounded_text_and_binary_round_trip(db: &mut TestDb<SharedArrayVecSchema>) {
                let SharedArrayVecSchema { rows, .. } = schema;

                // Empty, typical, full-capacity and multi-byte (15 bytes) text;
                // empty, typical and full-capacity binary.
                let names = ["", "Hello", "1234567890123456", "こんにちは"];
                let payloads: [Vec<u8>; 4] =
                    [vec![], vec![1, 2, 3, 4, 5], (0..32).collect(), vec![0; 20]];
                for (index, (name, payload)) in names.iter().zip(&payloads).enumerate() {
                    db.insert(rows)
                        .value(
                            InsertArrayRow::new(
                                ArrayString::<16>::from(name).unwrap(),
                                bytes::<32>(payload.iter().copied()),
                                format!("row_{index}"),
                            )
                            .with_id(index as i32 + 1),
                        )
                        .execute();
                }

                let stored: Vec<SelectArrayRow> =
                    db.select(()).from(rows).order_by(asc(rows.id)).all();
                assert_eq!(stored.len(), 4);
                for (index, row) in stored.iter().enumerate() {
                    assert_eq!(row.name.as_str(), names[index]);
                    assert_eq!(row.data.as_slice(), payloads[index].as_slice());
                    assert_eq!(row.label, format!("row_{index}"));
                }
                assert_eq!(stored[2].name.len(), 16);
                assert_eq!(stored[2].data.len(), 32);

                let matched: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(rows.label, "row_1"))
                    .all();
                assert_eq!(matched, [2]);
            }

            #[drizzle::test($dialect)]
            fn bounded_columns_update_in_place(db: &mut TestDb<SharedArrayVecSchema>) {
                let SharedArrayVecSchema { rows, .. } = schema;
                db.insert(rows)
                    .value(
                        InsertArrayRow::new(
                            ArrayString::<16>::from("before").unwrap(),
                            bytes::<32>([1, 2, 3]),
                            "update test",
                        )
                        .with_id(1),
                    )
                    .execute();

                db.update(rows)
                    .set(
                        UpdateArrayRow::default()
                            .with_name(ArrayString::<16>::from("after").unwrap())
                            .with_data(bytes::<32>([10, 20, 30, 40])),
                    )
                    .r#where(eq(rows.id, 1))
                    .execute();

                let stored: SelectArrayRow = db.select(()).from(rows).get();
                assert_eq!(stored.name.as_str(), "after");
                assert_eq!(stored.data.as_slice(), &[10, 20, 30, 40]);
            }

            #[drizzle::test($dialect)]
            fn different_capacities_coexist(db: &mut TestDb<SharedArrayVecSchema>) {
                let SharedArrayVecSchema { mixed, .. } = schema;
                let long_name = "This is a much longer name that fits in 64 chars";
                db.insert(mixed)
                    .value(
                        InsertMixedRow::new(
                            ArrayString::<8>::from("Short").unwrap(),
                            ArrayString::<64>::from(long_name).unwrap(),
                            bytes::<16>([1, 2, 3, 4, 5]),
                            bytes::<128>((0..100).map(|i| i as u8)),
                        )
                        .with_id(1),
                    )
                    .execute();

                let stored: SelectMixedRow = db.select(()).from(mixed).get();
                assert_eq!(stored.short_name.as_str(), "Short");
                assert_eq!(stored.long_name.as_str(), long_name);
                assert_eq!(stored.small_data.as_slice(), &[1, 2, 3, 4, 5]);
                assert_eq!(stored.large_data.len(), 100);
                assert!(
                    stored
                        .large_data
                        .iter()
                        .enumerate()
                        .all(|(i, b)| *b == i as u8)
                );
            }

            #[drizzle::test($dialect)]
            fn nullable_bounded_columns(db: &mut TestDb<SharedArrayVecSchema>) {
                let SharedArrayVecSchema { nullable, .. } = schema;
                db.insert(nullable)
                    .value(
                        InsertNullableRow::new()
                            .with_id(1)
                            .with_name(ArrayString::<16>::from("Some Name").unwrap())
                            .with_data(bytes::<32>([10, 20, 30])),
                    )
                    .execute();
                db.insert(nullable)
                    .value(InsertNullableRow::new().with_id(2))
                    .execute();

                let stored: Vec<SelectNullableRow> = db
                    .select(())
                    .from(nullable)
                    .order_by(asc(nullable.id))
                    .all();
                assert_eq!(stored.len(), 2);
                assert_eq!(stored[0].name.as_ref().unwrap().as_str(), "Some Name");
                assert_eq!(stored[0].data.as_ref().unwrap().as_slice(), &[10, 20, 30]);
                assert!(stored[1].name.is_none());
                assert!(stored[1].data.is_none());
            }
        }
    };
}

pub(crate) use shared_arrayvec_suite;
