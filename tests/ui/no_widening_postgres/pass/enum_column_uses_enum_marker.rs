use drizzle::core::expr::Expr;
use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
enum Role {
    #[default]
    User,
    Admin,
}

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    #[column(enum)]
    role: Role,
}

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
}

trait IsEnumMarker {}
impl IsEnumMarker for drizzle::postgres::types::Enum {}

fn require_enum_marker<'a, E>(_expr: E)
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: IsEnumMarker,
{
}

fn main() {
    let Schema { users } = Schema::new();
    require_enum_marker(users.role);
}
