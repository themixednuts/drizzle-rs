use drizzle::core::expr::{all, any, count, gt, lt, window};
use drizzle::sqlite::prelude::*;

fn main() {
    // A condition list folds Aggregate across its elements exactly as chained
    // and()/or() calls would: any aggregate element makes the whole list Agg.
    let conjunction = all::<SQLiteValue, _>((
        gt::<SQLiteValue, _, _>(count(()), 5i64),
        lt::<SQLiteValue, _, _>(count(()), 100i64),
    ));
    let _ = conjunction.over(window());

    let disjunction = any::<SQLiteValue, _>((
        gt::<SQLiteValue, _, _>(count(()), 5i64),
        lt::<SQLiteValue, _, _>(count(()), 100i64),
    ));
    let _ = disjunction.over(window());
}
