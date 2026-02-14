use super::{Generator, RngCore, SeedValue};
use rand::Rng;

/// Generates UUID v4 strings (random, formatted as 8-4-4-4-12 hex).
pub struct UuidGen;

impl Generator for UuidGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
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
        SeedValue::Text(s)
    }
    fn name(&self) -> &'static str {
        "Uuid"
    }
}

/// Generates simple JSON objects with random key-value pairs.
pub struct JsonGen;

impl Generator for JsonGen {
    fn generate(&self, rng: &mut dyn RngCore, index: usize, _sql_type: &str) -> SeedValue {
        let num_fields = rng.random_range(1usize..=5);
        let mut fields = Vec::with_capacity(num_fields);
        for i in 0..num_fields {
            let val: i64 = rng.random_range(0..1000);
            fields.push(format!("\"field_{i}\": {val}"));
        }
        // Add an index field for traceability
        fields.push(format!("\"_index\": {index}"));
        SeedValue::Text(format!("{{{}}}", fields.join(", ")))
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
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let mut bytes = vec![0u8; self.size];
        rng.fill(&mut bytes[..]);
        SeedValue::Blob(bytes)
    }
    fn name(&self) -> &'static str {
        "Blob"
    }
}

/// Generates PostgreSQL-compatible INET values.
pub struct InetGen;

impl Generator for InetGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let b: u8 = rng.random_range(0..=255);
        let c: u8 = rng.random_range(0..=255);
        let d: u8 = rng.random_range(1..=254);
        SeedValue::Text(format!("10.{b}.{c}.{d}"))
    }
    fn name(&self) -> &'static str {
        "PgInet"
    }
}

/// Generates PostgreSQL-compatible CIDR values.
pub struct CidrGen;

impl Generator for CidrGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let b: u8 = rng.random_range(0..=255);
        let c: u8 = rng.random_range(0..=255);
        SeedValue::Text(format!("10.{b}.{c}.0/24"))
    }
    fn name(&self) -> &'static str {
        "PgCidr"
    }
}

/// Generates PostgreSQL-compatible MACADDR values.
pub struct MacAddrGen;

impl Generator for MacAddrGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let mut bytes = [0u8; 6];
        rng.fill(&mut bytes);
        SeedValue::Text(format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
        ))
    }
    fn name(&self) -> &'static str {
        "PgMacAddr"
    }
}

/// Generates PostgreSQL-compatible MACADDR8 values.
pub struct MacAddr8Gen;

impl Generator for MacAddr8Gen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let mut bytes = [0u8; 8];
        rng.fill(&mut bytes);
        SeedValue::Text(format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
        ))
    }
    fn name(&self) -> &'static str {
        "PgMacAddr8"
    }
}

/// Generates PostgreSQL-compatible POINT values.
pub struct PointGen;

impl Generator for PointGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x: i16 = rng.random_range(-1000..=1000);
        let y: i16 = rng.random_range(-1000..=1000);
        SeedValue::Text(format!("({x},{y})"))
    }
    fn name(&self) -> &'static str {
        "PgPoint"
    }
}

/// Generates PostgreSQL-compatible LINE values.
pub struct LineGen;

impl Generator for LineGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x1: i16 = rng.random_range(-1000..=1000);
        let y1: i16 = rng.random_range(-1000..=1000);
        let x2: i16 = x1 + rng.random_range(1..=25);
        let y2: i16 = y1 + rng.random_range(1..=25);
        SeedValue::Text(format!("[({x1},{y1}),({x2},{y2})]"))
    }
    fn name(&self) -> &'static str {
        "PgLine"
    }
}

/// Generates PostgreSQL-compatible LSEG values.
pub struct LsegGen;

impl Generator for LsegGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x1: i16 = rng.random_range(-1000..=1000);
        let y1: i16 = rng.random_range(-1000..=1000);
        let x2: i16 = x1 + rng.random_range(1..=25);
        let y2: i16 = y1 + rng.random_range(1..=25);
        SeedValue::Text(format!("[({x1},{y1}),({x2},{y2})]"))
    }
    fn name(&self) -> &'static str {
        "PgLseg"
    }
}

/// Generates PostgreSQL-compatible BOX values.
pub struct BoxGen;

impl Generator for BoxGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x1: i16 = rng.random_range(-1000..=950);
        let y1: i16 = rng.random_range(-1000..=950);
        let x2: i16 = x1 + rng.random_range(1..=50);
        let y2: i16 = y1 + rng.random_range(1..=50);
        SeedValue::Text(format!("(({x1},{y1}),({x2},{y2}))"))
    }
    fn name(&self) -> &'static str {
        "PgBox"
    }
}

/// Generates PostgreSQL-compatible PATH values.
pub struct PathGen;

impl Generator for PathGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x: i16 = rng.random_range(-100..=100);
        let y: i16 = rng.random_range(-100..=100);
        SeedValue::Text(
            format!(
                "[({x},{y}),({},{}) ,({},{}),({},{})]",
                x + 10,
                y + 5,
                x + 20,
                y,
                x + 30,
                y + 15
            )
            .replace(" ,", ","),
        )
    }
    fn name(&self) -> &'static str {
        "PgPath"
    }
}

/// Generates PostgreSQL-compatible POLYGON values.
pub struct PolygonGen;

impl Generator for PolygonGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x: i16 = rng.random_range(-100..=100);
        let y: i16 = rng.random_range(-100..=100);
        SeedValue::Text(
            format!(
                "(({x},{y}),({},{}) ,({},{}),({},{}) )",
                x + 10,
                y,
                x + 10,
                y + 10,
                x,
                y + 10
            )
            .replace(" ,", ",")
            .replace(") ", ")"),
        )
    }
    fn name(&self) -> &'static str {
        "PgPolygon"
    }
}

/// Generates PostgreSQL-compatible CIRCLE values.
pub struct CircleGen;

impl Generator for CircleGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let x: i16 = rng.random_range(-1000..=1000);
        let y: i16 = rng.random_range(-1000..=1000);
        let r: u8 = rng.random_range(1..=25);
        SeedValue::Text(format!("<({x},{y}),{r}>"))
    }
    fn name(&self) -> &'static str {
        "PgCircle"
    }
}

/// Generates PostgreSQL-compatible BIT and VARBIT values.
pub struct BitGen {
    pub min_len: usize,
    pub max_len: usize,
}

impl Generator for BitGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let len = rng.random_range(self.min_len..=self.max_len);
        let bits: String = (0..len)
            .map(|_| if rng.random_bool(0.5) { '1' } else { '0' })
            .collect();
        SeedValue::Text(bits)
    }
    fn name(&self) -> &'static str {
        "PgBit"
    }
}

/// Generates an empty PostgreSQL array literal.
pub struct ArrayGen;

impl Generator for ArrayGen {
    fn generate(&self, _rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Text("{}".to_string())
    }
    fn name(&self) -> &'static str {
        "PgArray"
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
            match g.generate(&mut rng, 0, "UUID") {
                SeedValue::Text(s) => {
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
            match g.generate(&mut rng, i, "JSONB") {
                SeedValue::Text(s) => {
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
            match g.generate(&mut rng, 0, "BLOB") {
                SeedValue::Blob(bytes) => {
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
        assert_eq!(
            g.generate(&mut rng1, 0, "BLOB"),
            g.generate(&mut rng2, 0, "BLOB")
        );
    }

    #[test]
    fn blob_different_seeds_produce_different_data() {
        let g = BlobGen { size: 32 };
        let mut rng1 = StdRng::seed_from_u64(1);
        let mut rng2 = StdRng::seed_from_u64(2);
        assert_ne!(
            g.generate(&mut rng1, 0, "BLOB"),
            g.generate(&mut rng2, 0, "BLOB")
        );
    }

    #[test]
    fn postgres_network_and_geo_shapes() {
        let mut rng = StdRng::seed_from_u64(42);

        let inet = InetGen.generate(&mut rng, 0, "INET");
        let cidr = CidrGen.generate(&mut rng, 0, "CIDR");
        let mac = MacAddrGen.generate(&mut rng, 0, "MACADDR");
        let point = PointGen.generate(&mut rng, 0, "POINT");

        match inet {
            SeedValue::Text(v) => assert!(v.contains('.')),
            _ => panic!("expected Text"),
        }
        match cidr {
            SeedValue::Text(v) => assert!(v.contains('/')),
            _ => panic!("expected Text"),
        }
        match mac {
            SeedValue::Text(v) => assert_eq!(v.matches(':').count(), 5),
            _ => panic!("expected Text"),
        }
        match point {
            SeedValue::Text(v) => assert!(v.starts_with('(') && v.ends_with(')')),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn bit_and_array_literals() {
        let mut rng = StdRng::seed_from_u64(42);
        let bit = BitGen {
            min_len: 4,
            max_len: 4,
        }
        .generate(&mut rng, 0, "BIT(4)");
        let arr = ArrayGen.generate(&mut rng, 0, "TEXT[]");

        match bit {
            SeedValue::Text(v) => {
                assert_eq!(v.len(), 4);
                assert!(v.chars().all(|c| c == '0' || c == '1'));
            }
            _ => panic!("expected Text"),
        }
        assert_eq!(arr, SeedValue::Text("{}".to_string()));
    }
}
