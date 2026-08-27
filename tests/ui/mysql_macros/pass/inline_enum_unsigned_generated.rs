use drizzle::mysql::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, MySQLEnum)]
enum State {
    Draft,
    Published,
}

#[MySQLTable(SCHEMA = "app_db", NAME = "documents")]
struct Documents {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(ENUM)]
    state: State,
    sequence: u32,
    #[column(generated(STORED, "sequence + 1"))]
    next_sequence: u32,
}

fn main() {
    let _insert = InsertDocuments::new(State::Draft, 1_u32);
    let _update = UpdateDocuments::default().with_state(State::Published);
    let _select = SelectDocuments {
        id: 1_u64,
        state: State::Published,
        sequence: 1_u32,
        next_sequence: 2_u32,
    };

    use drizzle::mysql::traits::MySQLEnum as _;
    let _: &'static str = State::SQL_TYPE;
    let _: &'static [&'static str] = State::VARIANTS;
    let _ = "Draft".parse::<State>();
}
