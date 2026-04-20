use std::collections::HashMap;

use syn::{Attribute, DataEnum, Expr, ExprLit, ExprUnary, Lit, UnOp, spanned::Spanned};

/// Parse a discriminant expression into an i64 value.
///
/// Handles positive literals (`3`), negative literals (`-1`), and
/// returns a compile error for anything else.
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
pub fn has_explicit_discriminants(data: &DataEnum) -> bool {
    data.variants.iter().any(|v| v.discriminant.is_some())
}

/// Check if the type has a `#[repr(iN)]` or `#[repr(uN)]` attribute.
pub fn has_integer_repr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("repr") {
            return false;
        }
        let Ok(nested) = attr.parse_args::<syn::Ident>() else {
            return false;
        };
        matches!(
            nested.to_string().as_str(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
        )
    })
}

/// Resolve all discriminant values for an enum's variants and validate uniqueness.
///
/// Returns a vector of `(variant_ident, discriminant_value)` pairs.
/// Emits a compile error if any two variants share the same discriminant.
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
