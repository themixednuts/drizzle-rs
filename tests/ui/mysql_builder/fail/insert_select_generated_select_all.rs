use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    title: String,
    #[column(generated(STORED, "CHAR_LENGTH(title)"))]
    title_len: u32,
}

#[derive(MySQLSchema)]
struct Schema {
    posts: Posts,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { posts } = Schema::new();
    let selected = builder.select(()).from(posts);
    let _ = builder.insert(posts).select(selected);
}
