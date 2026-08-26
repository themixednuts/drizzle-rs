use super::Numeric;

/// Compatibility marker for dialects whose arithmetic promotion does not
/// depend on the operator.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ArithmeticOp;

/// Type-level marker for SQL addition.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AddOp;

/// Type-level marker for SQL subtraction.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SubOp;

/// Type-level marker for SQL multiplication.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MulOp;

/// Type-level marker for SQL division.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DivOp;

/// Type-level marker for SQL remainder.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RemOp;

/// Arithmetic nullability follows the operands.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PropagateNullability;

/// Arithmetic can produce `NULL` independently of operand nullability.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AlwaysNullable;

/// Type-level policy for arithmetic nullability.
#[doc(hidden)]
pub trait ArithmeticNullability: super::private::Sealed + Copy + 'static {}

impl super::private::Sealed for PropagateNullability {}
impl super::private::Sealed for AlwaysNullable {}
impl ArithmeticNullability for PropagateNullability {}
impl ArithmeticNullability for AlwaysNullable {}

/// Maps a pair of numeric SQL types and an operator to the result SQL type.
///
/// The output follows SQL's type promotion rules: narrower types widen to
/// wider types (e.g. `Int2 + Int8 → Int8`, `Int4 + Float8 → Float8`).
/// Dialects whose output varies by operator, such as MySQL integer division,
/// implement only the corresponding operator marker.
#[diagnostic::on_unimplemented(
    message = "arithmetic between `{Self}` and `{Rhs}` is not supported",
    label = "both operands must be Numeric (Int, BigInt, Float, Double, etc.)"
)]
pub trait ArithmeticOutput<Rhs: Numeric = Self, Op = ArithmeticOp>: Numeric {
    /// The resulting SQL type of the arithmetic expression.
    type Output: Numeric;

    /// Whether the operator itself can introduce `NULL`.
    type Nullability: ArithmeticNullability;
}

/// Maps a numeric SQL type to the result type of unary negation.
#[diagnostic::on_unimplemented(
    message = "unary negation of `{Self}` is not supported",
    label = "the dialect has no numeric result mapping for this operand"
)]
pub trait NegOutput: Numeric {
    /// The resulting SQL type of `-expr`.
    type Output: Numeric;
}

macro_rules! neg_output {
    ($input:ty => $out:ty) => {
        impl NegOutput for $input {
            type Output = $out;
        }
    };
}

/// Implements the operator-independent compatibility form and every concrete
/// arithmetic operator for a dialect/type pair.
macro_rules! arithmetic_output {
    ($lhs:ty, $rhs:ty => $out:ty) => {
        impl ArithmeticOutput<$rhs> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, AddOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, SubOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, MulOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, DivOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, RemOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }
    };
}

macro_rules! sqlite_arithmetic_output {
    ($lhs:ty, $rhs:ty => $out:ty) => {
        impl ArithmeticOutput<$rhs> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, AddOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, SubOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, MulOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }

        impl ArithmeticOutput<$rhs, DivOp> for $lhs {
            type Output = $out;
            type Nullability = AlwaysNullable;
        }

        impl ArithmeticOutput<$rhs, RemOp> for $lhs {
            type Output = $out;
            type Nullability = AlwaysNullable;
        }
    };
}

// =============================================================================
// SQLite arithmetic output
// =============================================================================
//
// SQLite has only 3 numeric storage classes: Integer, Real, Numeric.
// Integer + Integer → Integer, Real + anything → Real, etc.

use crate::sqlite::types::{Integer, Numeric as SqliteNumeric, Real};

// Integer op Integer → Integer
sqlite_arithmetic_output!(Integer, Integer => Integer);
// Integer op Real → Real (widens to float)
sqlite_arithmetic_output!(Integer, Real => Real);
// Integer op Numeric → Numeric
sqlite_arithmetic_output!(Integer, SqliteNumeric => SqliteNumeric);

// Real op Integer → Real
sqlite_arithmetic_output!(Real, Integer => Real);
// Real op Real → Real
sqlite_arithmetic_output!(Real, Real => Real);
// Real op Numeric → Real
sqlite_arithmetic_output!(Real, SqliteNumeric => Real);

// Numeric op Integer → Numeric
sqlite_arithmetic_output!(SqliteNumeric, Integer => SqliteNumeric);
// Numeric op Real → Real (widens to float)
sqlite_arithmetic_output!(SqliteNumeric, Real => Real);
// Numeric op Numeric → Numeric
sqlite_arithmetic_output!(SqliteNumeric, SqliteNumeric => SqliteNumeric);

// SQLite Any ↔ all SQLite numeric types
use crate::sqlite::types::Any as SqliteAny;

arithmetic_output!(SqliteAny, SqliteAny => SqliteAny);
arithmetic_output!(SqliteAny, Integer => SqliteAny);
arithmetic_output!(SqliteAny, Real => SqliteAny);
arithmetic_output!(SqliteAny, SqliteNumeric => SqliteAny);
arithmetic_output!(Integer, SqliteAny => SqliteAny);
arithmetic_output!(Real, SqliteAny => SqliteAny);
arithmetic_output!(SqliteNumeric, SqliteAny => SqliteAny);

neg_output!(Integer => Integer);
neg_output!(Real => Real);
neg_output!(SqliteNumeric => SqliteNumeric);
neg_output!(SqliteAny => SqliteAny);

// =============================================================================
// PostgreSQL arithmetic output
// =============================================================================
//
// PostgreSQL type promotion lattice:
//   Int2 < Int4 < Int8 < Numeric
//   Float4 < Float8
//   Int + Float → Float (cross-family always widens to float)
//   Any integer + Numeric → Numeric
//   Any float + Numeric → Numeric (Float8)

use crate::postgres::types::{Float4, Float8, Int2, Int4, Int8, Numeric as PgNumeric};

/// Helper macro: generates `ArithmeticOutput` impls for a pair of PG types.
/// `wider!(A, B => W)` means A op B → W.
macro_rules! pg_arith {
    ($lhs:ty, $rhs:ty => $out:ty) => {
        arithmetic_output!($lhs, $rhs => $out);
    };
}

// --- Int2 (SMALLINT) ---
pg_arith!(Int2, Int2 => Int2);
pg_arith!(Int2, Int4 => Int4); // widens to Int4
pg_arith!(Int2, Int8 => Int8); // widens to Int8
pg_arith!(Int2, Float4 => Float4); // cross-family → float
pg_arith!(Int2, Float8 => Float8); // cross-family → float
pg_arith!(Int2, PgNumeric => PgNumeric);

// --- Int4 (INTEGER) ---
pg_arith!(Int4, Int2 => Int4); // Int4 is wider
pg_arith!(Int4, Int4 => Int4);
pg_arith!(Int4, Int8 => Int8); // widens to Int8
pg_arith!(Int4, Float4 => Float8); // cross-family → Float8 (PG rule)
pg_arith!(Int4, Float8 => Float8); // cross-family → Float8
pg_arith!(Int4, PgNumeric => PgNumeric);

// --- Int8 (BIGINT) ---
pg_arith!(Int8, Int2 => Int8); // Int8 is wider
pg_arith!(Int8, Int4 => Int8); // Int8 is wider
pg_arith!(Int8, Int8 => Int8);
pg_arith!(Int8, Float4 => Float8); // cross-family → Float8
pg_arith!(Int8, Float8 => Float8); // cross-family → Float8
pg_arith!(Int8, PgNumeric => PgNumeric);

// --- Float4 (REAL) ---
pg_arith!(Float4, Int2 => Float4); // float absorbs int
pg_arith!(Float4, Int4 => Float8); // PG: float4 + int4 → float8
pg_arith!(Float4, Int8 => Float8); // PG: float4 + int8 → float8
pg_arith!(Float4, Float4 => Float4);
pg_arith!(Float4, Float8 => Float8); // widens to Float8
pg_arith!(Float4, PgNumeric => Float8);

// --- Float8 (DOUBLE PRECISION) ---
pg_arith!(Float8, Int2 => Float8);
pg_arith!(Float8, Int4 => Float8);
pg_arith!(Float8, Int8 => Float8);
pg_arith!(Float8, Float4 => Float8); // Float8 is wider
pg_arith!(Float8, Float8 => Float8);
pg_arith!(Float8, PgNumeric => Float8);

// --- Numeric (NUMERIC/DECIMAL) ---
pg_arith!(PgNumeric, Int2 => PgNumeric);
pg_arith!(PgNumeric, Int4 => PgNumeric);
pg_arith!(PgNumeric, Int8 => PgNumeric);
pg_arith!(PgNumeric, Float4 => Float8); // PG casts numeric+float → float8
pg_arith!(PgNumeric, Float8 => Float8);
pg_arith!(PgNumeric, PgNumeric => PgNumeric);

neg_output!(Int2 => Int2);
neg_output!(Int4 => Int4);
neg_output!(Int8 => Int8);
neg_output!(Float4 => Float4);
neg_output!(Float8 => Float8);
neg_output!(PgNumeric => PgNumeric);

// =============================================================================
// MySQL arithmetic output
// =============================================================================
//
// MySQL 8.0 evaluates integer +, -, and * as BIGINT. An unsigned integer
// operand makes those results unsigned. Integer % keeps the left operand's
// signedness. Exact-value division produces DECIMAL, while any
// approximate-value operand makes the result DOUBLE.

use crate::mysql::types::{
    BigInt as MyBigInt, BigIntUnsigned as MyBigIntUnsigned, Decimal as MyDecimal,
    Double as MyDouble,
};

macro_rules! mysql_operator_output {
    ($lhs:ty, $rhs:ty => $out:ty) => {
        impl ArithmeticOutput<$rhs, AddOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }
        impl ArithmeticOutput<$rhs, SubOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }
        impl ArithmeticOutput<$rhs, MulOp> for $lhs {
            type Output = $out;
            type Nullability = PropagateNullability;
        }
    };
}

macro_rules! mysql_operator_row {
    ($lhs:ty, [$($rhs:ty),+ $(,)?] => $out:ty) => {
        $(mysql_operator_output!($lhs, $rhs => $out);)+
    };
}

macro_rules! mysql_operator_matrix {
    ([$($lhs:ty),+ $(,)?], $rhs:tt => $out:ty) => {
        $(mysql_operator_row!($lhs, $rhs => $out);)+
    };
}

macro_rules! mysql_division {
    ($lhs:ty, $rhs:ty => $out:ty) => {
        impl ArithmeticOutput<$rhs, DivOp> for $lhs {
            type Output = $out;
            type Nullability = AlwaysNullable;
        }
    };
}

macro_rules! mysql_remainder {
    ($lhs:ty, $rhs:ty => $out:ty) => {
        impl ArithmeticOutput<$rhs, RemOp> for $lhs {
            type Output = $out;
            type Nullability = AlwaysNullable;
        }
    };
}

macro_rules! mysql_remainder_row {
    ($lhs:ty, [$($rhs:ty),+ $(,)?] => $out:ty) => {
        $(mysql_remainder!($lhs, $rhs => $out);)+
    };
}

macro_rules! mysql_remainder_matrix {
    ([$($lhs:ty),+ $(,)?], $rhs:tt => $out:ty) => {
        $(mysql_remainder_row!($lhs, $rhs => $out);)+
    };
}

macro_rules! mysql_division_row {
    ($lhs:ty, [$($rhs:ty),+ $(,)?] => $out:ty) => {
        $(mysql_division!($lhs, $rhs => $out);)+
    };
}

macro_rules! mysql_division_matrix {
    ([$($lhs:ty),+ $(,)?], $rhs:tt => $out:ty) => {
        $(mysql_division_row!($lhs, $rhs => $out);)+
    };
}

macro_rules! mysql_arithmetic_matrices {
    (
        signed: [$($signed:ty),+ $(,)?],
        unsigned: [$($unsigned:ty),+ $(,)?],
        decimal: $decimal:ty,
        approximate: [$($approximate:ty),+ $(,)?],
    ) => {
        mysql_operator_matrix!([$($signed),+], [$($signed),+] => MyBigInt);
        mysql_operator_matrix!([$($signed),+], [$($unsigned),+] => MyBigIntUnsigned);
        mysql_operator_matrix!([$($unsigned),+], [$($signed),+, $($unsigned),+] => MyBigIntUnsigned);

        mysql_operator_matrix!([$($signed),+, $($unsigned),+], [$decimal] => MyDecimal);
        mysql_operator_matrix!([$decimal], [$($signed),+, $($unsigned),+, $decimal] => MyDecimal);

        mysql_operator_matrix!([
            $($signed),+, $($unsigned),+, $decimal
        ], [$($approximate),+] => MyDouble);
        mysql_operator_matrix!([
            $($approximate),+
        ], [$($signed),+, $($unsigned),+, $decimal, $($approximate),+] => MyDouble);

        mysql_remainder_matrix!([$($signed),+], [$($signed),+, $($unsigned),+] => MyBigInt);
        mysql_remainder_matrix!([$($unsigned),+], [$($signed),+, $($unsigned),+] => MyBigIntUnsigned);
        mysql_remainder_matrix!([$($signed),+, $($unsigned),+], [$decimal] => MyDecimal);
        mysql_remainder_matrix!([$decimal], [$($signed),+, $($unsigned),+, $decimal] => MyDecimal);
        mysql_remainder_matrix!([
            $($signed),+, $($unsigned),+, $decimal
        ], [$($approximate),+] => MyDouble);
        mysql_remainder_matrix!([
            $($approximate),+
        ], [$($signed),+, $($unsigned),+, $decimal, $($approximate),+] => MyDouble);

        mysql_division_matrix!([
            $($signed),+, $($unsigned),+, $decimal
        ], [$($signed),+, $($unsigned),+, $decimal] => MyDecimal);
        mysql_division_matrix!([
            $($signed),+, $($unsigned),+, $decimal
        ], [$($approximate),+] => MyDouble);
        mysql_division_matrix!([
            $($approximate),+
        ], [$($signed),+, $($unsigned),+, $decimal, $($approximate),+] => MyDouble);
    };
}

mysql_arithmetic_matrices! {
    signed: [
        crate::mysql::types::TinyInt,
        crate::mysql::types::SmallInt,
        crate::mysql::types::MediumInt,
        crate::mysql::types::Int,
        crate::mysql::types::BigInt,
    ],
    unsigned: [
        crate::mysql::types::TinyIntUnsigned,
        crate::mysql::types::SmallIntUnsigned,
        crate::mysql::types::MediumIntUnsigned,
        crate::mysql::types::IntUnsigned,
        crate::mysql::types::BigIntUnsigned,
        crate::mysql::types::Year,
    ],
    decimal: crate::mysql::types::Decimal,
    approximate: [crate::mysql::types::Float, crate::mysql::types::Double],
}

macro_rules! mysql_neg_outputs {
    (integers: [$($integer:ty),+ $(,)?], decimal: $decimal:ty, approximate: [$($approximate:ty),+ $(,)?] $(,)?) => {
        $(neg_output!($integer => MyBigInt);)+
        neg_output!($decimal => MyDecimal);
        $(neg_output!($approximate => MyDouble);)+
    };
}

mysql_neg_outputs! {
    integers: [
        crate::mysql::types::TinyInt,
        crate::mysql::types::SmallInt,
        crate::mysql::types::MediumInt,
        crate::mysql::types::Int,
        crate::mysql::types::BigInt,
        crate::mysql::types::TinyIntUnsigned,
        crate::mysql::types::SmallIntUnsigned,
        crate::mysql::types::MediumIntUnsigned,
        crate::mysql::types::IntUnsigned,
        crate::mysql::types::BigIntUnsigned,
        crate::mysql::types::Year,
    ],
    decimal: crate::mysql::types::Decimal,
    approximate: [crate::mysql::types::Float, crate::mysql::types::Double],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mysql::types as my;
    use crate::postgres::types as pg;
    use crate::sqlite::types as sqlite;

    trait Same<T> {}
    impl<T> Same<T> for T {}

    fn assert_output<Lhs, Rhs, Op, Output, Nullability>()
    where
        Lhs: ArithmeticOutput<Rhs, Op, Output = Output>,
        Rhs: Numeric,
        Output: Numeric,
        <Lhs as ArithmeticOutput<Rhs, Op>>::Nullability: Same<Nullability>,
    {
    }

    fn assert_neg_output<Input, Output>()
    where
        Input: NegOutput<Output = Output>,
        Output: Numeric,
    {
    }

    #[test]
    fn mysql_operator_result_types_follow_server_categories() {
        assert_output::<my::Int, my::SmallInt, AddOp, my::BigInt, PropagateNullability>();
        assert_output::<my::Int, my::IntUnsigned, SubOp, my::BigIntUnsigned, PropagateNullability>(
        );
        assert_output::<my::BigIntUnsigned, my::Int, MulOp, my::BigIntUnsigned, PropagateNullability>(
        );
        assert_output::<my::Int, my::Int, DivOp, my::Decimal, AlwaysNullable>();
        assert_output::<my::Int, my::IntUnsigned, RemOp, my::BigInt, AlwaysNullable>();
        assert_output::<my::IntUnsigned, my::Int, RemOp, my::BigIntUnsigned, AlwaysNullable>();
        assert_output::<my::Decimal, my::Int, AddOp, my::Decimal, PropagateNullability>();
        assert_output::<my::Float, my::Int, AddOp, my::Double, PropagateNullability>();
        assert_output::<my::Int, my::Double, DivOp, my::Double, AlwaysNullable>();
    }

    #[test]
    fn every_mysql_numeric_marker_has_operator_and_negation_policy() {
        macro_rules! assert_numeric_policy {
            ($($ty:ty),+ $(,)?) => {
                $(
                    assert_output::<$ty, $ty, AddOp, _, PropagateNullability>();
                    assert_output::<$ty, $ty, DivOp, _, AlwaysNullable>();
                    assert_output::<$ty, $ty, RemOp, _, AlwaysNullable>();
                    assert_neg_output::<$ty, _>();
                )+
            };
        }

        assert_numeric_policy!(
            my::TinyInt,
            my::TinyIntUnsigned,
            my::SmallInt,
            my::SmallIntUnsigned,
            my::MediumInt,
            my::MediumIntUnsigned,
            my::Int,
            my::IntUnsigned,
            my::BigInt,
            my::BigIntUnsigned,
            my::Year,
            my::Decimal,
            my::Float,
            my::Double,
        );
    }

    #[test]
    fn legacy_operator_independent_projection_remains_available() {
        fn assert_legacy<Lhs, Rhs, Output>()
        where
            Lhs: ArithmeticOutput<Rhs, Output = Output>,
            Rhs: Numeric,
            Output: Numeric,
        {
        }

        assert_legacy::<sqlite::Integer, sqlite::Real, sqlite::Real>();
        assert_legacy::<pg::Int4, pg::Float8, pg::Float8>();
    }

    #[test]
    fn sqlite_zero_divisor_operators_are_nullable() {
        assert_output::<sqlite::Integer, sqlite::Integer, DivOp, sqlite::Integer, AlwaysNullable>();
        assert_output::<sqlite::Integer, sqlite::Integer, RemOp, sqlite::Integer, AlwaysNullable>();
        assert_output::<sqlite::Real, sqlite::Integer, DivOp, sqlite::Real, AlwaysNullable>();
    }

    #[test]
    fn mysql_unary_negation_widens_to_a_signed_result() {
        assert_neg_output::<my::TinyInt, my::BigInt>();
        assert_neg_output::<my::BigIntUnsigned, my::BigInt>();
        assert_neg_output::<my::Decimal, my::Decimal>();
        assert_neg_output::<my::Float, my::Double>();
    }
}
