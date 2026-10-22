use drizzle::sqlite::prelude::*;

#[derive(SQLiteEnum, Clone, Copy, Debug, PartialEq)]
#[sqlite_enum(text)]
enum Shade {
    Light,
    Dark,
}

fn main() {}
