use crate::cli::Parity;
use crate::code::{Code, Fail};
use crate::load;

type FieldCheck = (&'static str, fn(&serde_json::Value) -> bool);

pub async fn run(args: Parity) -> Result<Code, Fail> {
    let target = load::resolve_text(args.target, "BENCH_TARGET_ID", "--target")?;
    let seed = load::resolve_num(args.seed, "BENCH_SEED", "--seed").unwrap_or(42);
    let trial = load::resolve_num(args.trial, "BENCH_TRIAL", "--trial").unwrap_or(1);

    let handle = load::serve_target(&target, seed, trial).await?;
    let port = handle.port;

    let result = tokio::task::spawn_blocking(move || check_all(port))
        .await
        .map_err(|err| Fail::new(Code::ParityFail, format!("parity panicked: {err}")))?;

    handle.shutdown().await?;
    result
}

fn check_all(port: u16) -> Result<Code, Fail> {
    check_stats(port)?;
    let customers = check_customers(port)?;
    check_customer_by_id(port, &customers)?;
    check_orders(port)?;
    check_orders_with_details(port)?;
    eprintln!("parity: all checks passed");
    Ok(Code::Success)
}

fn get_json(port: u16, path: &str) -> Result<serde_json::Value, Fail> {
    let (status, body) = load::send_get_body(port, path)
        .map_err(|err| Fail::new(Code::ParityFail, format!("parity {path}: {err}")))?;
    if status != 200 {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected 200, got {status}"),
        ));
    }
    serde_json::from_str(&body).map_err(|err| {
        Fail::new(
            Code::ParityFail,
            format!("parity {path}: invalid json: {err}"),
        )
    })
}

fn expect_array<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a Vec<serde_json::Value>, Fail> {
    value.as_array().ok_or_else(|| {
        Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected array, got {}", kind(value)),
        )
    })
}

fn kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn check_fields(obj: &serde_json::Value, path: &str, fields: &[FieldCheck]) -> Result<(), Fail> {
    let map = obj.as_object().ok_or_else(|| {
        Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected object, got {}", kind(obj)),
        )
    })?;
    for &(name, check) in fields {
        let val = map.get(name).ok_or_else(|| {
            Fail::new(
                Code::ParityFail,
                format!("parity {path}: missing field \"{name}\""),
            )
        })?;
        if !check(val) {
            return Err(Fail::new(
                Code::ParityFail,
                format!(
                    "parity {path}: field \"{name}\" has wrong type: {}",
                    kind(val)
                ),
            ));
        }
    }
    Ok(())
}

fn is_number(v: &serde_json::Value) -> bool {
    v.is_number()
}

fn is_string(v: &serde_json::Value) -> bool {
    v.is_string()
}

// ---------------------------------------------------------------------------
// Endpoint checks
// ---------------------------------------------------------------------------

fn check_stats(port: u16) -> Result<(), Fail> {
    let val = get_json(port, "/stats")?;
    let arr = expect_array(&val, "/stats")?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            "parity /stats: expected non-empty cpu array",
        ));
    }
    for item in arr {
        let item: &serde_json::Value = item;
        if !item.is_number() {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity /stats: expected number, got {}", kind(item)),
            ));
        }
    }
    Ok(())
}

/// Returns the parsed customers array for cross-check with customer-by-id.
fn check_customers(port: u16) -> Result<Vec<serde_json::Value>, Fail> {
    let path = "/customers?idx=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let user_fields: &[FieldCheck] =
        &[("id", is_number), ("name", is_string), ("email", is_string)];
    for row in arr {
        check_fields(row, path, user_fields)?;
    }
    Ok(arr.clone())
}

fn check_customer_by_id(port: u16, customers: &[serde_json::Value]) -> Result<(), Fail> {
    // Use the first customer's id for a known lookup
    let first_id = customers[0]
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            Fail::new(
                Code::ParityFail,
                "parity /customer-by-id: cannot extract id from customers",
            )
        })?;

    let path = format!("/customer-by-id?id={first_id}");
    let val = get_json(port, &path)?;
    let arr = expect_array(&val, &path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty result for known id"),
        ));
    }
    let user_fields: &[FieldCheck] =
        &[("id", is_number), ("name", is_string), ("email", is_string)];
    for row in arr {
        check_fields(row, &path, user_fields)?;
        let row_id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(-1);
        if row_id != first_id {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity {path}: expected id={first_id}, got id={row_id}"),
            ));
        }
    }
    Ok(())
}

fn check_orders(port: u16) -> Result<(), Fail> {
    let path = "/orders?idx=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let post_fields: &[FieldCheck] = &[
        ("id", is_number),
        ("title", is_string),
        ("author_id", is_number),
    ];
    for row in arr {
        check_fields(row, path, post_fields)?;
    }
    Ok(())
}

fn check_orders_with_details(port: u16) -> Result<(), Fail> {
    let path = "/orders-with-details";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let detail_fields: &[FieldCheck] = &[("name", is_string), ("title", is_string)];
    for row in arr {
        check_fields(row, path, detail_fields)?;
    }
    Ok(())
}
