use super::{BigInt, Double, Float, Int, Numeric, SmallInt};

pub trait ArithmeticOutput<Rhs: Numeric = Self>: Numeric {
    type Output: Numeric;
}

impl ArithmeticOutput<SmallInt> for SmallInt {
    type Output = SmallInt;
}
impl ArithmeticOutput<Int> for SmallInt {
    type Output = Int;
}
impl ArithmeticOutput<BigInt> for SmallInt {
    type Output = BigInt;
}

impl ArithmeticOutput<SmallInt> for Int {
    type Output = Int;
}
impl ArithmeticOutput<Int> for Int {
    type Output = Int;
}
impl ArithmeticOutput<BigInt> for Int {
    type Output = BigInt;
}

impl ArithmeticOutput<SmallInt> for BigInt {
    type Output = BigInt;
}
impl ArithmeticOutput<Int> for BigInt {
    type Output = BigInt;
}
impl ArithmeticOutput<BigInt> for BigInt {
    type Output = BigInt;
}

impl ArithmeticOutput<Float> for Float {
    type Output = Float;
}
impl ArithmeticOutput<Double> for Float {
    type Output = Double;
}
impl ArithmeticOutput<Float> for Double {
    type Output = Double;
}
impl ArithmeticOutput<Double> for Double {
    type Output = Double;
}

impl ArithmeticOutput<Float> for SmallInt {
    type Output = Float;
}
impl ArithmeticOutput<Double> for SmallInt {
    type Output = Double;
}

impl ArithmeticOutput<Float> for Int {
    type Output = Float;
}
impl ArithmeticOutput<Double> for Int {
    type Output = Double;
}

impl ArithmeticOutput<Float> for BigInt {
    type Output = Float;
}
impl ArithmeticOutput<Double> for BigInt {
    type Output = Double;
}

impl ArithmeticOutput<SmallInt> for Float {
    type Output = Float;
}
impl ArithmeticOutput<Int> for Float {
    type Output = Float;
}
impl ArithmeticOutput<BigInt> for Float {
    type Output = Float;
}

impl ArithmeticOutput<SmallInt> for Double {
    type Output = Double;
}
impl ArithmeticOutput<Int> for Double {
    type Output = Double;
}
impl ArithmeticOutput<BigInt> for Double {
    type Output = Double;
}
