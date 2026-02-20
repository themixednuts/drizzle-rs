use super::Numeric;

pub trait ArithmeticOutput<Rhs: Numeric = Self>: Numeric {
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
