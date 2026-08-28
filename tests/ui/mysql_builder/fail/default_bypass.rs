use drizzle::mysql::builder::{QueryBuilder, SelectFromSet};

fn main() {
    let _ = QueryBuilder::<(), SelectFromSet>::default();
}
