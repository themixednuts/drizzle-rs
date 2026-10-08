use drizzle::sqlite::prelude::*;

#[SQLiteTable(UNIQUE(columns(org, code)))]
struct Teams {
    #[column(primary)]
    id: i64,
    org: i64,
    code: i64,
}

// A typo in the action would otherwise reach the CREATE TABLE.
#[SQLiteTable(FOREIGN_KEY(
    columns(org, team_code),
    references(Teams, org, code),
    on_delete = "casade"
))]
struct Members {
    #[column(primary)]
    id: i64,
    org: i64,
    team_code: i64,
}

fn main() {}
