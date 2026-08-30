use drizzle_types::Dialect;
use syn::{BinOp, Error, Expr, Lit, Result, UnOp};

pub(super) fn render(expr: &Expr, dialect: Dialect) -> Result<String> {
    match expr {
        Expr::Lit(expr) => literal(&expr.lit, dialect),
        Expr::Path(expr)
            if expr.qself.is_none()
                && expr
                    .path
                    .segments
                    .iter()
                    .all(|segment| segment.arguments.is_empty()) =>
        {
            Ok(expr
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("."))
        }
        Expr::Call(expr) => Ok(format!(
            "{}({})",
            render(&expr.func, dialect)?,
            expr.args
                .iter()
                .map(|argument| render(argument, dialect))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
        Expr::MethodCall(expr) if expr.turbofish.is_none() => Ok(format!(
            "{}.{}({})",
            render(&expr.receiver, dialect)?,
            expr.method,
            expr.args
                .iter()
                .map(|argument| render(argument, dialect))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
        Expr::Paren(expr) => Ok(format!("({})", render(&expr.expr, dialect)?)),
        Expr::Group(expr) => render(&expr.expr, dialect),
        Expr::Unary(expr) => {
            let operator = match expr.op {
                UnOp::Neg(_) => "-",
                UnOp::Not(_) => "NOT ",
                _ => return Err(unsupported(expr)),
            };
            Ok(format!("{operator}{}", render(&expr.expr, dialect)?))
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
            Ok(format!(
                "{} {operator} {}",
                render(&expr.left, dialect)?,
                render(&expr.right, dialect)?
            ))
        }
        Expr::Tuple(expr) => Ok(format!(
            "({})",
            expr.elems
                .iter()
                .map(|element| render(element, dialect))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
        _ => Err(unsupported(expr)),
    }
}

fn literal(literal: &Lit, dialect: Dialect) -> Result<String> {
    match literal {
        Lit::Str(value) => Ok(format!("'{}'", string(&value.value(), dialect))),
        Lit::Char(value) => Ok(format!("'{}'", string(&value.value().to_string(), dialect))),
        Lit::Int(value) => Ok(value.base10_digits().to_string()),
        Lit::Float(value) => Ok(value.base10_digits().to_string()),
        Lit::Bool(value) if dialect == Dialect::SQLite => Ok(i8::from(value.value()).to_string()),
        Lit::Bool(value) => Ok(if value.value() { "TRUE" } else { "FALSE" }.to_string()),
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

fn string(value: &str, dialect: Dialect) -> String {
    let value = if dialect == Dialect::MySQL {
        value.replace('\\', "\\\\")
    } else {
        value.to_string()
    };
    value.replace('\'', "''")
}

fn unsupported(expr: impl quote::ToTokens) -> Error {
    Error::new_spanned(expr, "unsupported DEFAULT expression")
}
