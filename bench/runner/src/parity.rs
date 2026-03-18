use crate::cli::Parity;
use crate::code::{Code, Fail};
use crate::load;

type FieldCheck = (&'static str, fn(&serde_json::Value) -> bool);

pub async fn run(args: Parity) -> Result<Code, Fail> {
    let target = load::resolve_text(args.target, "BENCH_TARGET_ID", "--target")?;
    let seed: u64 = load::resolve_num(args.seed, "BENCH_SEED", "--seed").unwrap_or(42);

    let handle = load::serve_target(&target, seed).await?;
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
    check_employees(port)?;
    check_suppliers(port)?;
    check_supplier_by_id(port)?;
    check_products(port)?;
    check_employee_with_recipient(port)?;
    check_product_with_supplier(port)?;
    check_orders_with_details(port)?;
    check_order_with_details(port)?;
    check_order_with_details_and_products(port)?;
    check_search_customer(port)?;
    check_search_product(port)?;
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

fn is_string_or_null(v: &serde_json::Value) -> bool {
    v.is_string() || v.is_null()
}

fn is_array(v: &serde_json::Value) -> bool {
    v.is_array()
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
        if !item.is_number() {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity /stats: expected number, got {}", kind(item)),
            ));
        }
    }
    Ok(())
}

fn check_customers(port: u16) -> Result<Vec<serde_json::Value>, Fail> {
    let path = "/customers?limit=50&offset=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("companyName", is_string),
        ("contactName", is_string),
        ("city", is_string),
        ("country", is_string),
        ("phone", is_string),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(arr.clone())
}

fn check_customer_by_id(port: u16, customers: &[serde_json::Value]) -> Result<(), Fail> {
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
    for row in arr {
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

fn check_employees(port: u16) -> Result<(), Fail> {
    let path = "/employees?limit=20&offset=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("lastName", is_string),
        ("title", is_string),
        ("birthDate", is_number),
        ("hireDate", is_number),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_suppliers(port: u16) -> Result<(), Fail> {
    let path = "/suppliers?limit=50&offset=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("companyName", is_string),
        ("contactName", is_string),
        ("city", is_string),
        ("country", is_string),
        ("phone", is_string),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_supplier_by_id(port: u16) -> Result<(), Fail> {
    let path = "/supplier-by-id?id=1";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty result"),
        ));
    }
    Ok(())
}

fn check_products(port: u16) -> Result<(), Fail> {
    let path = "/products?limit=50&offset=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("name", is_string),
        ("unitPrice", is_number),
        ("supplierId", is_number),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_employee_with_recipient(port: u16) -> Result<(), Fail> {
    let path = "/employee-with-recipient?id=2";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("lastName", is_string),
        ("recipientLastName", is_string_or_null),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_product_with_supplier(port: u16) -> Result<(), Fail> {
    let path = "/product-with-supplier?id=1";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("name", is_string),
        ("supplierId", is_number),
        ("supplier", |v| v.is_object()),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_orders_with_details(port: u16) -> Result<(), Fail> {
    let path = "/orders-with-details?limit=50&offset=0";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("shipName", is_string),
        ("productsCount", is_number),
        ("totalPrice", is_number),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_order_with_details(port: u16) -> Result<(), Fail> {
    let path = "/order-with-details?id=1";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("orderDate", is_number),
        ("freight", is_number),
        ("details", is_array),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_order_with_details_and_products(port: u16) -> Result<(), Fail> {
    let path = "/order-with-details-and-products?id=1";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if arr.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("orderDate", is_number),
        ("details", is_array),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
        // Check that details have productName
        if let Some(details) = row.get("details").and_then(|v| v.as_array())
            && let Some(first) = details.first()
        {
            check_fields(
                first,
                &format!("{path}/details[0]"),
                &[
                    ("unitPrice", is_number),
                    ("quantity", is_number),
                    ("productName", is_string),
                ],
            )?;
        }
    }
    Ok(())
}

fn check_search_customer(port: u16) -> Result<(), Fail> {
    let path = "/search-customer?term=er";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if let Some(first) = arr.first() {
        check_fields(
            first,
            path,
            &[("id", is_number), ("companyName", is_string)],
        )?;
    }
    Ok(())
}

fn check_search_product(port: u16) -> Result<(), Fail> {
    let path = "/search-product?term=er";
    let val = get_json(port, path)?;
    let arr = expect_array(&val, path)?;
    if let Some(first) = arr.first() {
        check_fields(first, path, &[("id", is_number), ("name", is_string)])?;
    }
    Ok(())
}
