use drizzle::mysql::prelude::*;

struct MyUuid;

#[MySQLTable]
struct LookalikeTypesAreNotInferred {
    value: MyUuid,
}

fn main() {}
