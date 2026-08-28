use drizzle_types::Dialect;
use syn::{BinOp, Error, Expr, Lit, Result, UnOp};

/// Render the Rust-like expression accepted by `DEFAULT` as SQL.
///
/// String literals become SQL string literals. Other expressions retain their
/// expression semantics, which lets `DEFAULT = CURRENT_TIMESTAMP` and
/// `DEFAULT = now()` remain unquoted.
pub fn render_default(expr: &Expr, dialect: Dialect) -> Result<String> {
    match expr {
        Expr::Lit(expr) => render_literal(&expr.lit, dialect),
        Expr::Path(expr) => {
            if expr.qself.is_some()
                || expr
                    .path
                    .segments
                    .iter()
                    .any(|segment| !segment.arguments.is_empty())
            {
                return Err(unsupported(expr));
            }
            Ok(expr
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("."))
        }
        Expr::Call(expr) => {
            let function = render_default(&expr.func, dialect)?;
            let arguments = expr
                .args
                .iter()
                .map(|argument| render_default(argument, dialect))
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            Ok(format!("{function}({arguments})"))
        }
        Expr::MethodCall(expr) if expr.turbofish.is_none() => {
            let receiver = render_default(&expr.receiver, dialect)?;
            let arguments = expr
                .args
                .iter()
                .map(|argument| render_default(argument, dialect))
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            let method = &expr.method;
            Ok(format!("{receiver}.{method}({arguments})"))
        }
        Expr::Paren(expr) => Ok(format!("({})", render_default(&expr.expr, dialect)?)),
        Expr::Group(expr) => render_default(&expr.expr, dialect),
        Expr::Unary(expr) => {
            let operator = match expr.op {
                UnOp::Neg(_) => "-",
                UnOp::Not(_) => "NOT ",
                _ => return Err(unsupported(expr)),
            };
            Ok(format!(
                "{operator}{}",
                render_default(&expr.expr, dialect)?
            ))
        }
        Expr::Binary(expr) => {
            let operator = match expr.op {
                BinOp::Add(_) => "+",
                BinOp::Sub(_) => "-",
                BinOp::Mul(_) => "*",
                BinOp::Div(_) => "/",
                BinOp::Rem(_) => "%",
                BinOp::And(_) => "AND",
                BinOp::Or(_) => "OR",
                BinOp::BitXor(_) => "^",
                BinOp::BitAnd(_) => "&",
                BinOp::BitOr(_) => "|",
                BinOp::Shl(_) => "<<",
                BinOp::Shr(_) => ">>",
                BinOp::Eq(_) => "=",
                BinOp::Lt(_) => "<",
                BinOp::Le(_) => "<=",
                BinOp::Ne(_) => "<>",
                BinOp::Ge(_) => ">=",
                BinOp::Gt(_) => ">",
                _ => return Err(unsupported(expr)),
            };
            let left = render_default(&expr.left, dialect)?;
            let right = render_default(&expr.right, dialect)?;
            Ok(format!("{left} {operator} {right}"))
        }
        Expr::Tuple(expr) => {
            let elements = expr
                .elems
                .iter()
                .map(|element| render_default(element, dialect))
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            Ok(format!("({elements})"))
        }
        _ => Err(unsupported(expr)),
    }
}

fn render_literal(literal: &Lit, dialect: Dialect) -> Result<String> {
    match literal {
        Lit::Str(value) => Ok(format!("'{}'", escape_string(&value.value(), dialect))),
        Lit::Char(value) => Ok(format!(
            "'{}'",
            escape_string(&value.value().to_string(), dialect)
        )),
        Lit::Int(value) => Ok(value.base10_digits().to_string()),
        Lit::Float(value) => Ok(value.base10_digits().to_string()),
        Lit::Bool(value) => match dialect {
            Dialect::SQLite => Ok(i8::from(value.value()).to_string()),
            Dialect::PostgreSQL | Dialect::MySQL => {
                Ok(if value.value() { "TRUE" } else { "FALSE" }.to_string())
            }
        },
        Lit::Byte(value) if dialect == Dialect::MySQL => Ok(value.value().to_string()),
        Lit::ByteStr(value) if dialect == Dialect::MySQL => Ok(format!(
            "X'{}'",
            value
                .value()
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        )),
        _ => Err(Error::new_spanned(
            literal,
            "unsupported DEFAULT literal for this SQL dialect",
        )),
    }
}

fn escape_string(value: &str, dialect: Dialect) -> String {
    let value = if dialect == Dialect::MySQL {
        value.replace('\\', "\\\\")
    } else {
        value.to_string()
    };
    value.replace('\'', "''")
}

fn unsupported(expr: impl quote::ToTokens) -> Error {
    Error::new_spanned(
        expr,
        "unsupported DEFAULT expression; use a literal, SQL keyword, function call, or SQL operator expression",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str, dialect: Dialect) -> String {
        render_default(&syn::parse_str(source).unwrap(), dialect).unwrap()
    }

    #[test]
    fn distinguishes_strings_from_sql_expressions() {
        assert_eq!(parse(r#""guest""#, Dialect::PostgreSQL), "'guest'");
        assert_eq!(
            parse("CURRENT_TIMESTAMP", Dialect::SQLite),
            "CURRENT_TIMESTAMP"
        );
        assert_eq!(parse("now()", Dialect::PostgreSQL), "now()");
        assert_eq!(
            parse(r#"strftime("%s", "now")"#, Dialect::SQLite),
            "strftime('%s', 'now')"
        );
    }

    #[test]
    fn applies_dialect_literal_rules() {
        assert_eq!(parse("true", Dialect::SQLite), "1");
        assert_eq!(parse("true", Dialect::PostgreSQL), "TRUE");
        assert_eq!(parse("b\"hi\"", Dialect::MySQL), "X'6869'");
        assert_eq!(parse(r#""it's""#, Dialect::MySQL), "'it''s'");
    }
}
