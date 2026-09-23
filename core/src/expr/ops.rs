//! Arithmetic operations using `std::ops` traits.
//!
//! This module implements `Add`, `Sub`, `Mul`, `Div`, `Rem` for `SQLExpr`,
//! enabling natural Rust syntax for SQL arithmetic.

use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use crate::dialect::Dialect;
use crate::sql::{SQL, SQLChunk, Token};
use crate::traits::SQLParam;
use crate::types::{AddOp, ArithmeticOutput, DivOp, MulOp, NegOutput, Numeric, RemOp, SubOp};

use super::{AggOr, AggregateKind, Expr, Nullability, ResolveArithmeticNullability, SQLExpr};

type ArithmeticNullable<'a, V, T, N, Rhs, Op> = <<T as ArithmeticOutput<
    <Rhs as Expr<'a, V>>::SQLType,
    Op,
>>::Nullability as ResolveArithmeticNullability<
    N,
    <Rhs as Expr<'a, V>>::Nullable,
>>::Output;

#[inline]
fn binary_op_sql<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
{
    binary_operator_sql(left.into_expr_sql(), operator, right.into_expr_sql())
}

// =============================================================================
// Operator precedence
// =============================================================================

/// Rank given to anything between two operands that is not a ranked binary
/// operator: comparisons, logical operators, raw operator text. It is below
/// every ranked operator, so such an operand is always grouped.
const LOOSEST: u8 = 0;

/// How tightly `operator` binds between two operands in `dialect`; larger
/// binds tighter. Only the operators the expression builders place between
/// two operands are ranked.
const fn binding_power(dialect: Dialect, operator: Token) -> u8 {
    match dialect {
        // SQLite ranks `||` above `*`, and `& | << >>` together below `+ -`.
        Dialect::SQLite => match operator {
            Token::CONCAT => 5,
            Token::STAR | Token::SLASH | Token::REM => 4,
            Token::PLUS | Token::MINUS => 3,
            Token::BITAND | Token::BITOR | Token::LSHIFT | Token::RSHIFT => 2,
            _ => LOOSEST,
        },
        // PostgreSQL puts `||` and the bitwise operators in its shared
        // "any other operator" level, below `+ -`.
        Dialect::PostgreSQL => match operator {
            Token::STAR | Token::SLASH | Token::REM => 4,
            Token::PLUS | Token::MINUS => 3,
            Token::CONCAT | Token::BITAND | Token::BITOR | Token::LSHIFT | Token::RSHIFT => 2,
            _ => LOOSEST,
        },
        // MySQL reads `||` as logical OR, so it stays unranked.
        Dialect::MySQL => match operator {
            Token::STAR | Token::SLASH | Token::REM => 6,
            Token::PLUS | Token::MINUS => 5,
            Token::LSHIFT | Token::RSHIFT => 4,
            Token::BITAND => 3,
            Token::BITOR => 2,
            _ => LOOSEST,
        },
    }
}

/// The loosest-binding operator found at the top level of an operand.
#[derive(Clone, Copy)]
struct TopLevelOperator {
    power: u8,
    /// Every operator at `power` is `||`, which is associative.
    only_concat: bool,
}

impl TopLevelOperator {
    fn record(found: &mut Option<Self>, power: u8, concat: bool) {
        match found {
            None => {
                *found = Some(Self {
                    power,
                    only_concat: concat,
                });
            }
            Some(current) if power < current.power => {
                *current = Self {
                    power,
                    only_concat: concat,
                };
            }
            Some(current) if power == current.power => current.only_concat &= concat,
            Some(_) => {}
        }
    }
}

/// Keyword and comparison tokens that join two operands at a looser level
/// than any arithmetic operator.
const fn is_loose_infix(token: Token) -> bool {
    matches!(
        token,
        Token::EQ
            | Token::NE
            | Token::LT
            | Token::GT
            | Token::LE
            | Token::GE
            | Token::AND
            | Token::OR
            | Token::NOT
            | Token::IS
            | Token::ISNOT
            | Token::IN
            | Token::LIKE
            | Token::BETWEEN
            | Token::ESCAPE
            | Token::ISNULL
            | Token::NOTNULL
            | Token::MATCH
    )
}

/// Raw text that reads as a single term (a function name, keyword literal,
/// or number) rather than as an operator.
fn is_term_text(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '.' | '"' | '`' | '\''))
}

/// Finds the loosest binary operator outside any parentheses or
/// `CASE ... END` in `operand`. `None` means the operand is a single term: a
/// column, value, function call, parenthesized group, or a sign-prefixed term.
fn top_level_operator<V: SQLParam>(operand: &SQL<'_, V>) -> Option<TopLevelOperator> {
    let mut depth = 0usize;
    // True at the start and after an operator, where `-`/`+` are signs.
    let mut expect_term = true;
    let mut found = None;

    for chunk in &operand.chunks {
        match chunk {
            SQLChunk::Token(Token::LPAREN | Token::CASE) => {
                depth += 1;
                expect_term = false;
            }
            SQLChunk::Token(Token::RPAREN | Token::END) => {
                depth = depth.saturating_sub(1);
                expect_term = false;
            }
            _ if depth > 0 => {}
            SQLChunk::Token(
                token @ (Token::PLUS
                | Token::MINUS
                | Token::STAR
                | Token::SLASH
                | Token::REM
                | Token::CONCAT
                | Token::BITAND
                | Token::BITOR
                | Token::LSHIFT
                | Token::RSHIFT),
            ) => {
                if !expect_term {
                    TopLevelOperator::record(
                        &mut found,
                        binding_power(V::DIALECT, *token),
                        matches!(token, Token::CONCAT),
                    );
                    expect_term = true;
                }
            }
            SQLChunk::Token(Token::BITNOT) => {}
            SQLChunk::Token(token) if is_loose_infix(*token) => {
                TopLevelOperator::record(&mut found, LOOSEST, false);
                expect_term = true;
            }
            SQLChunk::Raw(text) if expect_term && matches!(text.trim(), "-" | "+") => {}
            SQLChunk::Raw(text) if !is_term_text(text) => {
                TopLevelOperator::record(&mut found, LOOSEST, false);
                expect_term = true;
            }
            _ => expect_term = false,
        }
    }

    found
}

/// Whether `operand` must be parenthesized to stay a single operand of
/// `operator`. SQL operators of equal precedence associate to the left, so a
/// right-hand operand also needs grouping at equal precedence (`a - (b - c)`
/// is not `a - b - c`); the associative `||` is the exception.
fn needs_grouping<V: SQLParam>(operand: &SQL<'_, V>, operator: Token, right_hand: bool) -> bool {
    let Some(inner) = top_level_operator(operand) else {
        return false;
    };
    let outer = binding_power(V::DIALECT, operator);
    if right_hand {
        inner.power < outer
            || (inner.power == outer && !(matches!(operator, Token::CONCAT) && inner.only_concat))
    } else {
        inner.power < outer
    }
}

/// Renders `left operator right` so the database evaluates the tree the Rust
/// expression built.
///
/// An operand that is itself a binary expression is parenthesized when its
/// top-level operator binds more loosely than `operator` (on the right-hand
/// side, also when it binds equally), so `a * (b + c)` keeps its grouping.
/// Operands that already read correctly stay flat: a single term renders as
/// before, and so does a chain such as `a * b + c`.
pub(crate) fn binary_operator_sql<'a, V>(
    left: SQL<'a, V>,
    operator: Token,
    right: SQL<'a, V>,
) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    let left = left.parens_if_subquery();
    let right = right.parens_if_subquery();
    let left = if needs_grouping(&left, operator, false) {
        left.parens()
    } else {
        left
    };
    let right = if needs_grouping(&right, operator, true) {
        right.parens()
    } else {
        right
    };
    left.push(operator).append(right)
}

// =============================================================================
// Addition
// =============================================================================

impl<'a, V, T, N, A, Rhs> Add<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType, AddOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, AddOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, AddOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, AddOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn add(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(binary_op_sql(self, Token::PLUS, rhs))
    }
}

// =============================================================================
// Subtraction
// =============================================================================

impl<'a, V, T, N, A, Rhs> Sub<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType, SubOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, SubOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, SubOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, SubOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn sub(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(binary_op_sql(self, Token::MINUS, rhs))
    }
}

// =============================================================================
// Multiplication
// =============================================================================

impl<'a, V, T, N, A, Rhs> Mul<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType, MulOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, MulOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, MulOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, MulOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn mul(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(binary_op_sql(self, Token::STAR, rhs))
    }
}

// =============================================================================
// Division
// =============================================================================

impl<'a, V, T, N, A, Rhs> Div<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType, DivOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, DivOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, DivOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, DivOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn div(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(binary_op_sql(self, Token::SLASH, rhs))
    }
}

// =============================================================================
// Remainder (Modulo)
// =============================================================================

impl<'a, V, T, N, A, Rhs> Rem<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType, RemOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, RemOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, RemOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, RemOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn rem(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(binary_op_sql(self, Token::REM, rhs))
    }
}

// =============================================================================
// Negation
// =============================================================================

impl<'a, V, T, N, A> Neg for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: Numeric + NegOutput,
    N: Nullability,
    A: AggregateKind,
{
    type Output = SQLExpr<'a, V, T::Output, N, A>;

    fn neg(self) -> Self::Output {
        SQLExpr::new(SQL::from(Token::MINUS).append(self.into_expr_sql().parens()))
    }
}

#[cfg(test)]
mod tests {
    use super::binary_operator_sql;
    use crate::sql::{SQL, Token};
    use crate::{Dialect, MySQLDialect, PostgresDialect, SQLParam, SQLiteDialect};

    #[derive(Clone, Debug)]
    struct SqliteParam;

    impl SQLParam for SqliteParam {
        const DIALECT: Dialect = Dialect::SQLite;
        type DialectMarker = SQLiteDialect;
    }

    #[derive(Clone, Debug)]
    struct PostgresParam;

    impl SQLParam for PostgresParam {
        const DIALECT: Dialect = Dialect::PostgreSQL;
        type DialectMarker = PostgresDialect;
    }

    #[derive(Clone, Debug)]
    struct MySqlParam;

    impl SQLParam for MySqlParam {
        const DIALECT: Dialect = Dialect::MySQL;
        type DialectMarker = MySQLDialect;
    }

    fn term<V: SQLParam>(name: &'static str) -> SQL<'static, V> {
        SQL::ident(name)
    }

    fn apply<V: SQLParam>(
        left: SQL<'static, V>,
        operator: Token,
        right: SQL<'static, V>,
    ) -> SQL<'static, V> {
        binary_operator_sql(left, operator, right)
    }

    #[test]
    fn single_operator_stays_flat() {
        let product = apply::<SqliteParam>(term("a"), Token::STAR, term("b"));
        assert_eq!(product.sql(), r#""a" * "b""#);
    }

    #[test]
    fn looser_operand_is_grouped_on_either_side() {
        let sum = || apply::<SqliteParam>(term("b"), Token::PLUS, term("c"));
        assert_eq!(
            apply(term("a"), Token::STAR, sum()).sql(),
            r#""a" *("b" + "c")"#
        );
        assert_eq!(
            apply(sum(), Token::STAR, term("a")).sql(),
            r#"("b" + "c")* "a""#
        );
    }

    #[test]
    fn tighter_left_chain_stays_flat() {
        let product = apply::<PostgresParam>(term("a"), Token::STAR, term("b"));
        let chain = apply(product, Token::PLUS, term("c"));
        assert_eq!(chain.sql(), r#""a" * "b" + "c""#);

        let difference = apply::<PostgresParam>(term("a"), Token::MINUS, term("b"));
        let chain = apply(difference, Token::MINUS, term("c"));
        assert_eq!(chain.sql(), r#""a" - "b" - "c""#);
    }

    #[test]
    fn equal_precedence_on_the_right_is_grouped() {
        let difference = apply::<MySqlParam>(term("b"), Token::MINUS, term("c"));
        assert_eq!(
            apply(term("a"), Token::MINUS, difference).sql(),
            "`a` -(`b` - `c`)"
        );
    }

    #[test]
    fn concatenation_chain_stays_flat_on_the_right() {
        let tail = apply::<SqliteParam>(term("b"), Token::CONCAT, term("c"));
        assert_eq!(
            apply(term("a"), Token::CONCAT, tail).sql(),
            r#""a" || "b" || "c""#
        );
    }

    #[test]
    fn concatenation_precedence_follows_the_dialect() {
        // SQLite binds `||` tighter than `+`; PostgreSQL binds it looser.
        let sqlite_sum = apply::<SqliteParam>(term("b"), Token::PLUS, term("c"));
        assert_eq!(
            apply(term("a"), Token::CONCAT, sqlite_sum).sql(),
            r#""a" ||("b" + "c")"#
        );

        let postgres_sum = apply::<PostgresParam>(term("b"), Token::PLUS, term("c"));
        assert_eq!(
            apply(term("a"), Token::CONCAT, postgres_sum).sql(),
            r#""a" || "b" + "c""#
        );
    }

    #[test]
    fn terms_with_inner_operators_stay_flat() {
        // A function call, a parenthesized group and a signed term are each
        // a single operand, whatever they contain.
        let call = SQL::<SqliteParam>::raw("ABS")
            .push(Token::LPAREN)
            .append(apply(term("b"), Token::MINUS, term("c")))
            .push(Token::RPAREN);
        assert_eq!(
            apply(term("a"), Token::STAR, call).sql(),
            r#""a" * ABS ("b" - "c")"#
        );

        let signed = SQL::<SqliteParam>::raw("-").append(term("b"));
        assert_eq!(
            apply(term("a"), Token::MINUS, signed).sql(),
            r#""a" - - "b""#
        );
    }

    #[test]
    fn comparison_operand_is_grouped() {
        let comparison = SQL::<SqliteParam>::ident("b")
            .push(Token::EQ)
            .append(term("c"));
        assert_eq!(
            apply(term("a"), Token::PLUS, comparison).sql(),
            r#""a" +("b" = "c")"#
        );
    }
}
