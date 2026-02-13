use rand::SeedableRng;
use rand::rngs::StdRng;
use sha2::{Digest, Sha256};

/// Create a deterministic RNG for a specific table + column combination.
///
/// Uses SHA-256 to hash `"table.column"` into a 64-bit seed, then adds the
/// user-provided seed. This ensures each column gets a unique but reproducible
/// sequence of values.
pub fn column_rng(table: &str, column: &str, seed: u64) -> StdRng {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}.{}", table, column).as_bytes());
    let hash = hasher.finalize();
    // Take first 8 bytes of SHA-256 as u64
    let col_hash = u64::from_le_bytes(hash[..8].try_into().unwrap());
    StdRng::seed_from_u64(col_hash.wrapping_add(seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn deterministic_across_calls() {
        let mut rng1 = column_rng("users", "email", 42);
        let mut rng2 = column_rng("users", "email", 42);
        let vals1: Vec<u32> = (0..10).map(|_| rng1.random()).collect();
        let vals2: Vec<u32> = (0..10).map(|_| rng2.random()).collect();
        assert_eq!(vals1, vals2);
    }

    #[test]
    fn different_columns_produce_different_values() {
        let mut rng1 = column_rng("users", "email", 42);
        let mut rng2 = column_rng("users", "name", 42);
        let v1: u64 = rng1.random();
        let v2: u64 = rng2.random();
        assert_ne!(v1, v2);
    }

    #[test]
    fn different_seeds_produce_different_values() {
        let mut rng1 = column_rng("users", "email", 1);
        let mut rng2 = column_rng("users", "email", 2);
        let v1: u64 = rng1.random();
        let v2: u64 = rng2.random();
        assert_ne!(v1, v2);
    }
}
