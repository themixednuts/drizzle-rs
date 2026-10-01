use drizzle::core::expr::{eq, in_subquery};
use drizzle::sqlite::{builder::QueryBuilder, prelude::*};

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    // A DML statement with RETURNING is not a SELECT and cannot be a set operand.
    let delete = builder
        .delete(users)
        .r#where(eq(users.id, 1))
        .returning(users.id);
    let _ = delete.union(builder.select(users.id).from(users));
    let update = builder
        .update(users)
        .set(UpdateUsers::default().with_name("a"))
        .returning(users.id);
    let _ = builder.select(users.id).from(users).union(update);
    // Nor a subquery.
    let returning = builder.delete(users).returning(users.id);
    let _ = builder
        .select(users.id)
        .from(users)
        .r#where(in_subquery(users.id, returning));
}
