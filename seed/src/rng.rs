use rand::SeedableRng;
use rand::rngs::StdRng;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

const fn fnv1a_extend(mut state: u64, bytes: &[u8]) -> u64 {
    let mut i = 0;
    while i < bytes.len() {
        state ^= bytes[i] as u64;
        state = state.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    state
}

/// Deterministically derive a 64-bit seed for `table.column`.
pub const fn column_seed(table: &str, column: &str, seed: u64) -> u64 {
    let mut state = FNV_OFFSET_BASIS;
    state = fnv1a_extend(state, table.as_bytes());
    state = fnv1a_extend(state, b".");
    state = fnv1a_extend(state, column.as_bytes());
    state.wrapping_add(seed)
}

/// Create a deterministic RNG for a specific table + column combination.
///
/// Uses a const FNV-1a hash for `"table.column"`, then adds the user seed.
/// This ensures each column gets a unique but reproducible value sequence.
pub fn column_rng(table: &str, column: &str, seed: u64) -> StdRng {
    StdRng::seed_from_u64(column_seed(table, column, seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    const CONST_SEED: u64 = column_seed("users", "email", 42);

    #[test]
    fn column_seed_is_const_usable() {
        assert_eq!(CONST_SEED, column_seed("users", "email", 42));
    }

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
