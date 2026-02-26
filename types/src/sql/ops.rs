use super::Numeric;

/// Maps the left-hand numeric type to the result type of an arithmetic
/// operation (`+`, `-`, `*`, `/`, `%`).
///
/// The output preserves the left operand's type width: e.g. `Int4 + Int4 → Int4`.
/// This is a dialect-independent trait because arithmetic output rules are the
/// same across SQLite and PostgreSQL in this codebase.
#[diagnostic::on_unimplemented(
    message = "arithmetic between `{Self}` and `{Rhs}` is not supported",
    label = "both operands must be Numeric (Int, BigInt, Float, Double, etc.)"
)]
pub trait ArithmeticOutput<Rhs: Numeric = Self>: Numeric {
    /// The resulting SQL type of the arithmetic expression.
    type Output: Numeric;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::sqlite::types::Integer {
    type Output = crate::sqlite::types::Integer;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::sqlite::types::Real {
    type Output = crate::sqlite::types::Real;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::sqlite::types::Numeric {
    type Output = crate::sqlite::types::Numeric;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::postgres::types::Int2 {
    type Output = crate::postgres::types::Int2;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::postgres::types::Int4 {
    type Output = crate::postgres::types::Int4;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::postgres::types::Int8 {
    type Output = crate::postgres::types::Int8;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::postgres::types::Float4 {
    type Output = crate::postgres::types::Float4;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::postgres::types::Float8 {
    type Output = crate::postgres::types::Float8;
}

impl<Rhs: Numeric> ArithmeticOutput<Rhs> for crate::postgres::types::Numeric {
    type Output = crate::postgres::types::Numeric;
}
