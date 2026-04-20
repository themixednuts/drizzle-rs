use super::Numeric;

/// Maps the left-hand numeric type to the result type of an arithmetic
/// operation (`+`, `-`, `*`, `/`, `%`).
///
/// The output follows SQL's type promotion rules: narrower types widen to
/// wider types (e.g. `Int2 + Int8 → Int8`, `Int4 + Float8 → Float8`).
/// Within the same width, the left operand's type is preserved.
#[diagnostic::on_unimplemented(
    message = "arithmetic between `{Self}` and `{Rhs}` is not supported",
    label = "both operands must be Numeric (Int, BigInt, Float, Double, etc.)"
)]
pub trait ArithmeticOutput<Rhs: Numeric = Self>: Numeric {
    /// The resulting SQL type of the arithmetic expression.
    type Output: Numeric;
}

// =============================================================================
// SQLite arithmetic output
// =============================================================================
//
// SQLite has only 3 numeric storage classes: Integer, Real, Numeric.
// Integer + Integer → Integer, Real + anything → Real, etc.

use crate::sqlite::types::{Integer, Numeric as SqliteNumeric, Real};

// Integer op Integer → Integer
impl ArithmeticOutput<Self> for Integer {
    type Output = Self;
}
// Integer op Real → Real (widens to float)
impl ArithmeticOutput<Real> for Integer {
    type Output = Real;
}
// Integer op Numeric → Numeric
impl ArithmeticOutput<SqliteNumeric> for Integer {
    type Output = SqliteNumeric;
}

// Real op Integer → Real
impl ArithmeticOutput<Integer> for Real {
    type Output = Self;
}
// Real op Real → Real
impl ArithmeticOutput<Self> for Real {
    type Output = Self;
}
// Real op Numeric → Real
impl ArithmeticOutput<SqliteNumeric> for Real {
    type Output = Self;
}

// Numeric op Integer → Numeric
impl ArithmeticOutput<Integer> for SqliteNumeric {
    type Output = Self;
}
// Numeric op Real → Real (widens to float)
impl ArithmeticOutput<Real> for SqliteNumeric {
    type Output = Real;
}
// Numeric op Numeric → Numeric
impl ArithmeticOutput<Self> for SqliteNumeric {
    type Output = Self;
}

// SQLite Any ↔ all SQLite numeric types
use crate::sqlite::types::Any as SqliteAny;

impl ArithmeticOutput<Self> for SqliteAny {
    type Output = Self;
}
impl ArithmeticOutput<Integer> for SqliteAny {
    type Output = Self;
}
impl ArithmeticOutput<Real> for SqliteAny {
    type Output = Self;
}
impl ArithmeticOutput<SqliteNumeric> for SqliteAny {
    type Output = Self;
}
impl ArithmeticOutput<SqliteAny> for Integer {
    type Output = SqliteAny;
}
impl ArithmeticOutput<SqliteAny> for Real {
    type Output = SqliteAny;
}
impl ArithmeticOutput<SqliteAny> for SqliteNumeric {
    type Output = SqliteAny;
}

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
        impl ArithmeticOutput<$rhs> for $lhs {
            type Output = $out;
        }
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
