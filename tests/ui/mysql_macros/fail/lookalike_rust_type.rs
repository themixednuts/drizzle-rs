use drizzle::mysql::prelude::*;

#[derive(Clone, Debug)]
struct MyUuid;

#[MySQLTable]
struct LookalikeTypesAreNotInferred {
    value: MyUuid,
}

fn main() {}
