use super::{Generator, RngCore, SeedValue};
use rand::Rng;

const fn one_indexed_i64(index: usize) -> i64 {
    (index as i64) + 1
}

/// Generates sequential primary key values starting at 1.
pub struct IntPrimaryKeyGen;

impl Generator for IntPrimaryKeyGen {
    fn generate(&self, _rng: &mut dyn RngCore, index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Integer(one_indexed_i64(index))
    }
    fn name(&self) -> &'static str {
        "IntPrimaryKey"
    }
}

/// Generates random integers in [min, max].
pub struct IntGen {
    pub min: i64,
    pub max: i64,
}

impl Generator for IntGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Integer(rng.random_range(self.min..=self.max))
    }
    fn name(&self) -> &'static str {
        "Int"
    }
}

/// Generates random floating-point numbers in [min, max).
pub struct FloatGen {
    pub min: f64,
    pub max: f64,
}

impl Generator for FloatGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let v: f64 = rng.random_range(self.min..self.max);
        // Round to 2 decimal places
        SeedValue::Float((v * 100.0).round() / 100.0)
    }
    fn name(&self) -> &'static str {
        "Float"
    }
}

/// Generates random booleans.
pub struct BoolGen;

impl Generator for BoolGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Bool(rng.random_bool(0.5))
    }
    fn name(&self) -> &'static str {
        "Bool"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn pk_generates_sequential() {
        let g = IntPrimaryKeyGen;
        let mut rng = StdRng::seed_from_u64(0);
        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Integer(1));
        assert_eq!(g.generate(&mut rng, 1, "INTEGER"), SeedValue::Integer(2));
        assert_eq!(g.generate(&mut rng, 99, "INTEGER"), SeedValue::Integer(100));
    }

    #[test]
    fn int_in_range() {
        let g = IntGen { min: 10, max: 20 };
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            match g.generate(&mut rng, 0, "INTEGER") {
                SeedValue::Integer(v) => assert!((10..=20).contains(&v)),
                _ => panic!("expected Integer"),
            }
        }
    }

    #[test]
    fn bool_generates_both() {
        let g = BoolGen;
        let mut rng = StdRng::seed_from_u64(42);
        let vals: Vec<bool> = (0..100)
            .map(|i| match g.generate(&mut rng, i, "BOOLEAN") {
                SeedValue::Bool(v) => v,
                _ => panic!("expected Bool"),
            })
            .collect();
        assert!(vals.contains(&true));
        assert!(vals.contains(&false));
    }

    #[test]
    fn float_in_range_and_rounded() {
        let g = FloatGen {
            min: 1.0,
            max: 100.0,
        };
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            match g.generate(&mut rng, 0, "REAL") {
                SeedValue::Float(v) => {
                    assert!(v >= 1.0 && v < 100.0, "float out of range: {}", v);
                    // Should be rounded to 2 decimal places
                    let s = format!("{}", v);
                    if let Some(dot_pos) = s.find('.') {
                        let decimals = s.len() - dot_pos - 1;
                        assert!(decimals <= 2, "float has too many decimals: {}", s);
                    }
                }
                _ => panic!("expected Float"),
            }
        }
    }

    #[test]
    fn pk_is_one_indexed() {
        let g = IntPrimaryKeyGen;
        let mut rng = StdRng::seed_from_u64(0);
        // Index 0 → value 1 (one-indexed)
        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Integer(1));
        // Index 999 → value 1000
        assert_eq!(
            g.generate(&mut rng, 999, "INTEGER"),
            SeedValue::Integer(1000)
        );
    }
}
