use super::{Generator, RngCore, SqlValue};
use rand::Rng;

/// Generates UUID v4 strings (random, formatted as 8-4-4-4-12 hex).
pub struct UuidGen;

impl Generator for UuidGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);
        // Set version 4
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        // Set variant 1
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let s = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15],
        );
        SqlValue::Text(s)
    }
    fn name(&self) -> &'static str {
        "Uuid"
    }
}

/// Generates simple JSON objects with random key-value pairs.
pub struct JsonGen;

impl Generator for JsonGen {
    fn generate(&self, rng: &mut dyn RngCore, index: usize) -> SqlValue {
        let num_fields = rng.random_range(1usize..=5);
        let mut fields = Vec::with_capacity(num_fields);
        for i in 0..num_fields {
            let val: i64 = rng.random_range(0..1000);
            fields.push(format!("\"field_{i}\": {val}"));
        }
        // Add an index field for traceability
        fields.push(format!("\"_index\": {index}"));
        SqlValue::Text(format!("{{{}}}", fields.join(", ")))
    }
    fn name(&self) -> &'static str {
        "Json"
    }
}

/// Generates random binary blobs of a given size.
pub struct BlobGen {
    pub size: usize,
}

impl Generator for BlobGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let mut bytes = vec![0u8; self.size];
        rng.fill(&mut bytes[..]);
        SqlValue::Blob(bytes)
    }
    fn name(&self) -> &'static str {
        "Blob"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn uuid_v4_format_across_many_values() {
        let g = UuidGen;
        let mut rng = StdRng::seed_from_u64(42);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert_eq!(s.len(), 36, "uuid wrong length: {}", s);
                    assert_eq!(&s[8..9], "-");
                    assert_eq!(&s[13..14], "-");
                    assert_eq!(&s[14..15], "4", "uuid version not 4: {}", s);
                    assert_eq!(&s[18..19], "-");
                    assert_eq!(&s[23..24], "-");
                    // Variant bits: char at position 19 should be 8, 9, a, or b
                    let variant_char = s.chars().nth(19).unwrap();
                    assert!(
                        "89ab".contains(variant_char),
                        "uuid variant bits wrong (char '{}' at pos 19): {}",
                        variant_char,
                        s
                    );
                    // All chars should be hex or dash
                    assert!(
                        s.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
                        "uuid contains non-hex: {}",
                        s
                    );
                    // UUIDs should be unique
                    assert!(seen.insert(s.clone()), "duplicate uuid: {}", s);
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn json_structure_and_index() {
        let g = JsonGen;
        let mut rng = StdRng::seed_from_u64(42);
        for i in 0..20 {
            match g.generate(&mut rng, i) {
                SqlValue::Text(s) => {
                    assert!(s.starts_with('{'), "json doesn't start with '{{': {}", s);
                    assert!(s.ends_with('}'), "json doesn't end with '}}': {}", s);
                    assert!(
                        s.contains(&format!("\"_index\": {}", i)),
                        "json missing _index field for index {}: {}",
                        i,
                        s
                    );
                    // Should have at least the _index field and one random field
                    assert!(s.contains("\"field_"), "json missing field_ entries: {}", s);
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn blob_correct_size() {
        for size in [1, 16, 32, 128] {
            let g = BlobGen { size };
            let mut rng = StdRng::seed_from_u64(42);
            match g.generate(&mut rng, 0) {
                SqlValue::Blob(bytes) => {
                    assert_eq!(bytes.len(), size, "blob size mismatch for size={}", size);
                }
                _ => panic!("expected Blob"),
            }
        }
    }

    #[test]
    fn blob_is_deterministic() {
        let g = BlobGen { size: 32 };
        let mut rng1 = StdRng::seed_from_u64(42);
        let mut rng2 = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng1, 0), g.generate(&mut rng2, 0));
    }

    #[test]
    fn blob_different_seeds_produce_different_data() {
        let g = BlobGen { size: 32 };
        let mut rng1 = StdRng::seed_from_u64(1);
        let mut rng2 = StdRng::seed_from_u64(2);
        assert_ne!(g.generate(&mut rng1, 0), g.generate(&mut rng2, 0));
    }
}
