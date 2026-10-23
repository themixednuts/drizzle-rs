use std::collections::HashMap;

use syn::{Attribute, DataEnum, Expr, ExprLit, ExprUnary, Lit, UnOp, spanned::Spanned};

/// Parse a discriminant expression into an i64 value.
///
/// Handles positive literals (`3`), negative literals (`-1`), and
/// returns a compile error for anything else.
#[cfg_attr(not(any(feature = "sqlite", feature = "postgres")), allow(dead_code))]
pub fn parse_discriminant(expr: &Expr) -> syn::Result<i64> {
    match expr {
        // Simple positive literal like `3`
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i
            .base10_parse::<i64>()
            .map_err(|e| syn::Error::new(i.span(), e)),

        // Negative literal like `-1`
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => {
            if let Expr::Lit(ExprLit {
                lit: Lit::Int(i), ..
            }) = &**expr
            {
                let val = i
                    .base10_parse::<i64>()
                    .map_err(|e| syn::Error::new(i.span(), e))?;
                Ok(-val)
            } else {
                Err(syn::Error::new(
                    expr.span(),
                    "expected an integer literal after `-`; enum discriminants must be integer values",
                ))
            }
        }

        other => Err(syn::Error::new(
            other.span(),
            "enum discriminant must be an integer literal (e.g., `= 1` or `= -1`)",
        )),
    }
}

/// Check if the enum has any variant with an explicit discriminant (`= N`).
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub fn has_explicit_discriminants(data: &DataEnum) -> bool {
    data.variants.iter().any(|v| v.discriminant.is_some())
}

/// The integer type of a `#[repr(iN)]` or `#[repr(uN)]` attribute, if any.
#[cfg_attr(not(any(feature = "sqlite", feature = "postgres")), allow(dead_code))]
pub fn integer_repr(attrs: &[Attribute]) -> Option<syn::Ident> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("repr") {
            return None;
        }
        let nested = attr.parse_args::<syn::Ident>().ok()?;
        matches!(
            nested.to_string().as_str(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
        )
        .then_some(nested)
    })
}

/// Check if the type has a `#[repr(iN)]` or `#[repr(uN)]` attribute.
#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
pub fn has_integer_repr(attrs: &[Attribute]) -> bool {
    integer_repr(attrs).is_some()
}

/// Resolve all discriminant values for an enum's variants and validate uniqueness.
///
/// Returns a vector of `(variant_ident, discriminant_value)` pairs.
/// Emits a compile error if any two variants share the same discriminant.
#[cfg_attr(not(any(feature = "sqlite", feature = "postgres")), allow(dead_code))]
pub fn resolve_discriminants(data: &DataEnum) -> syn::Result<Vec<(&syn::Ident, i64)>> {
    let mut results = Vec::with_capacity(data.variants.len());
    // Track which value maps to which variant name (for error messages)
    let mut seen: HashMap<i64, &syn::Ident> = HashMap::new();
    let mut next_value: i64 = 0;

    for variant in &data.variants {
        let value = if let Some((_, expr)) = &variant.discriminant {
            parse_discriminant(expr)?
        } else {
            next_value
        };

        if let Some(prev_ident) = seen.get(&value) {
            return Err(syn::Error::new(
                variant.ident.span(),
                format!(
                    "duplicate discriminant value {}: variant `{}` conflicts with `{}`",
                    value, variant.ident, prev_ident,
                ),
            ));
        }

        seen.insert(value, &variant.ident);
        results.push((&variant.ident, value));
        next_value = value + 1;
    }

    Ok(results)
}

/// Whether the enum carries the `#[name]` helper attribute of another enum
/// derive.
///
/// An enum can derive more than one dialect's enum (`SQLiteEnum`,
/// `PostgresEnum`, `MySQLEnum`). Each derive generates the same
/// dialect-neutral impls (`Display`, `FromStr`, `TryFrom<&str>`, ...), and a
/// derive cannot see which other derives sit on the enum, so the impls are
/// emitted by one of them only: SQLite first, then Postgres, then MySQL. A
/// derive leaves them to a higher-priority derive whose helper attribute
/// (`#[sqlite_enum]`, `#[postgres_enum]`) is on the enum. A helper attribute
/// only compiles when its derive is present, so its presence proves the
/// other derive will emit them.
#[cfg_attr(not(any(feature = "postgres", feature = "mysql")), allow(dead_code))]
pub fn has_helper_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

/// Rejects arguments on a marker attribute such as `#[sqlite_enum]`.
#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
pub fn validate_marker_attribute(attrs: &[Attribute], name: &str) -> syn::Result<()> {
    for attr in attrs {
        if attr.path().is_ident(name) && !matches!(attr.meta, syn::Meta::Path(_)) {
            return Err(syn::Error::new_spanned(
                attr,
                format!("#[{name}] takes no arguments"),
            ));
        }
    }
    Ok(())
}
