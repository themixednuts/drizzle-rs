/// Round trips for the optional wrapper types (`compact_str`, `bytes`,
/// `smallvec`). Each test is gated on its cargo feature so the suite can be
/// instantiated unconditionally; SQLite additionally checks the storage class
/// these types land in (see `tests/sqlite/wrappers.rs`).
macro_rules! shared_wrapper_type_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_wrapper_types {
            #[allow(unused_imports)]
            use super::*;
            #[allow(unused_imports)]
            use drizzle::core::expr::eq;

            #[cfg(feature = "compact-str")]
            #[$table(NAME = "shared_wrapper_compact_strings")]
            struct CompactStringRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: compact_str::CompactString,
                note: String,
            }

            #[cfg(feature = "compact-str")]
            #[derive($schema)]
            struct CompactStringSchema {
                rows: CompactStringRow,
            }

            #[cfg(feature = "compact-str")]
            #[drizzle::test($dialect)]
            fn compact_strings_round_trip(db: &mut TestDb<CompactStringSchema>) {
                let CompactStringSchema { rows } = schema;

                let value = compact_str::CompactString::new("compact hello");
                db.insert(rows)
                    .value(InsertCompactStringRow::new(value.clone(), "compact note").with_id(1))
                    .execute();

                let stored: Vec<SelectCompactStringRow> =
                    db.select(()).from(rows).r#where(eq(rows.id, 1)).all();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0].name, value);
                assert_eq!(stored[0].note, "compact note");

                let matched: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(rows.name, "compact hello"))
                    .all();
                assert_eq!(matched, [1]);
            }

            #[cfg(feature = "bytes")]
            #[$table(NAME = "shared_wrapper_bytes")]
            struct BytesRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                payload: bytes::Bytes,
                mutable_payload: bytes::BytesMut,
                note: String,
            }

            #[cfg(feature = "bytes")]
            #[derive($schema)]
            struct BytesSchema {
                rows: BytesRow,
            }

            #[cfg(feature = "bytes")]
            #[drizzle::test($dialect)]
            fn bytes_round_trip(db: &mut TestDb<BytesSchema>) {
                let BytesSchema { rows } = schema;

                let payload = bytes::Bytes::from_static(b"hello-bytes");
                let mutable_payload = bytes::BytesMut::from(&b"hello-bytes-mut"[..]);
                db.insert(rows)
                    .value(
                        InsertBytesRow::new(payload.clone(), mutable_payload.clone(), "bytes note")
                            .with_id(1),
                    )
                    .execute();

                let stored: Vec<SelectBytesRow> = db.select(()).from(rows).all();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0].payload.as_ref(), payload.as_ref());
                assert_eq!(stored[0].mutable_payload.as_ref(), mutable_payload.as_ref());
                assert_eq!(stored[0].note, "bytes note");
            }

            #[cfg(feature = "smallvec-types")]
            #[$table(NAME = "shared_wrapper_smallvecs")]
            struct SmallVecRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                payload: smallvec::SmallVec<[u8; 16]>,
                note: String,
            }

            #[cfg(feature = "smallvec-types")]
            #[derive($schema)]
            struct SmallVecSchema {
                rows: SmallVecRow,
            }

            #[cfg(feature = "smallvec-types")]
            #[drizzle::test($dialect)]
            fn small_vectors_round_trip_inline_and_spilled(db: &mut TestDb<SmallVecSchema>) {
                let SmallVecSchema { rows } = schema;

                let mut inline = smallvec::SmallVec::<[u8; 16]>::new();
                inline.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
                let mut spilled = smallvec::SmallVec::<[u8; 16]>::new();
                spilled.extend((0..40).map(|i| i as u8));
                assert!(spilled.spilled());

                db.insert(rows)
                    .values([
                        InsertSmallVecRow::new(inline.clone(), "inline").with_id(1),
                        InsertSmallVecRow::new(spilled.clone(), "spilled").with_id(2),
                    ])
                    .execute();

                let stored: Vec<SelectSmallVecRow> =
                    db.select(()).from(rows).order_by(asc(rows.id)).all();
                assert_eq!(stored.len(), 2);
                assert_eq!(stored[0].payload.as_slice(), inline.as_slice());
                assert_eq!(stored[1].payload.as_slice(), spilled.as_slice());
                assert_eq!(stored[1].note, "spilled");
            }
        }
    };
}

pub(crate) use shared_wrapper_type_suite;
