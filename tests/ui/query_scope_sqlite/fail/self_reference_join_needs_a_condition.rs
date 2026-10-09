use drizzle::sqlite::builder::QueryBuilder;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Employees {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = Employees::id)]
    manager_id: Option<i32>,
}

#[derive(SQLiteSchema)]
struct Schema {
    employees: Employees,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { employees } = Schema::new();
    // The key could join either way round: manager or report?
    let _ = qb.select(employees.id).from(employees).join(employees);
}
