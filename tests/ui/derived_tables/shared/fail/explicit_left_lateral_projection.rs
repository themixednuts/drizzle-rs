use drizzle::core::{
    Cons, LeftLateralSelection, Nil, Scoped, SelectCols,
};

struct Users;
struct UserId;
struct DerivedTitle;

type ExplicitProjection = Scoped<SelectCols<(UserId, DerivedTitle)>, Cons<Users, Nil>>;

fn require_safe_left_lateral<Selection: LeftLateralSelection<Users>>() {}

fn main() {
    require_safe_left_lateral::<ExplicitProjection>();
}
