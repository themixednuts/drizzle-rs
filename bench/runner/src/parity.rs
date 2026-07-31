use crate::cli::Parity;
use crate::code::{Code, Fail};
use crate::jsonio;
use crate::load;
use crate::model::Target;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type FieldCheck = (&'static str, fn(&serde_json::Value) -> bool);

/// Page size every paginated parity probe asks for.
///
/// The seeded dataset has at least this many rows in every listed table, so a
/// short page means the target ignored `limit` or lost rows.
const PAGE: usize = 50;

/// Env var naming the file this parity run writes its canonical bodies to.
///
/// `run` sets it per target and then compares the files; a standalone
/// `bench-runner parity` invocation just skips the snapshot.
pub const SNAPSHOT_ENV: &str = "BENCH_PARITY_SNAPSHOT";

pub async fn run(args: Parity) -> Result<Code, Fail> {
    let target = load::resolve_text(args.target, "BENCH_TARGET_ID", "--target")?;
    let seed: u64 = load::resolve_num(args.seed, "BENCH_SEED", "--seed").unwrap_or(42);
    let snapshot_out = std::env::var_os(SNAPSHOT_ENV).map(PathBuf::from);

    let handle = load::serve_target(&target, seed).await?;
    let port = handle.port;

    let result = tokio::task::spawn_blocking(move || check_all(port))
        .await
        .map_err(|err| Fail::new(Code::ParityFail, format!("parity panicked: {err}")))?;

    handle.shutdown().await?;
    let snapshot = result?;

    if let Some(path) = snapshot_out {
        jsonio::write(path, &snapshot, Code::ParityFail)?;
    }
    eprintln!("parity: all checks passed");
    Ok(Code::Success)
}

/// One route's normalized response body, keyed by request path.
pub type Snapshot = BTreeMap<String, Value>;

fn check_all(port: u16) -> Result<Snapshot, Fail> {
    let mut snapshot = Snapshot::new();
    check_stats(port)?;
    let customers = check_customers(port, &mut snapshot)?;
    check_customer_by_id(port, &customers, &mut snapshot)?;
    check_employees(port, &mut snapshot)?;
    check_suppliers(port, &mut snapshot)?;
    check_supplier_by_id(port, &mut snapshot)?;
    check_products(port, &mut snapshot)?;
    check_employee_with_recipient(port, &mut snapshot)?;
    check_product_with_supplier(port, &mut snapshot)?;
    check_orders_with_details(port, &mut snapshot)?;
    check_order_with_details(port, &mut snapshot)?;
    check_order_with_details_and_products(port, &mut snapshot)?;
    check_search_customer(port, &mut snapshot)?;
    check_search_product(port, &mut snapshot)?;
    Ok(snapshot)
}

/// Fetch a route, record its canonical body, and hand back the parsed value.
fn probe(port: u16, path: &str, snapshot: &mut Snapshot) -> Result<Value, Fail> {
    let value = get_json(port, path)?;
    snapshot.insert(path.to_string(), canonicalize(&value));
    Ok(value)
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

/// Assert an array has exactly `want` elements.
fn expect_len(rows: &[Value], want: usize, path: &str) -> Result<(), Fail> {
    if rows.len() != want {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected {want} rows, got {}", rows.len()),
        ));
    }
    Ok(())
}

fn expect_non_empty<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, Fail> {
    let rows = expect_array(value, path)?;
    if rows.is_empty() {
        return Err(Fail::new(
            Code::ParityFail,
            format!("parity {path}: expected non-empty array"),
        ));
    }
    Ok(rows)
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

fn string_field<'a>(obj: &'a serde_json::Value, path: &str, field: &str) -> Result<&'a str, Fail> {
    obj.get(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            Fail::new(
                Code::ParityFail,
                format!("parity {path}: field \"{field}\" must be a string"),
            )
        })
}

fn number_field(obj: &Value, path: &str, field: &str) -> Result<f64, Fail> {
    obj.get(field).and_then(Value::as_f64).ok_or_else(|| {
        Fail::new(
            Code::ParityFail,
            format!("parity {path}: field \"{field}\" must be a number"),
        )
    })
}

fn id_field(obj: &Value, path: &str) -> Result<i64, Fail> {
    obj.get("id").and_then(Value::as_i64).ok_or_else(|| {
        Fail::new(
            Code::ParityFail,
            format!("parity {path}: row has no integer \"id\""),
        )
    })
}

fn contains_term(value: &str, term: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&term.to_ascii_lowercase())
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

fn is_date(v: &serde_json::Value) -> bool {
    v.is_number() || v.is_string()
}

fn is_array(v: &serde_json::Value) -> bool {
    v.is_array()
}

/// Floating point tolerance for aggregate cross-checks.
///
/// The aggregate may be summed by the database in a different order than the
/// detail rows are summed here, so exact equality is the wrong assertion.
const EPSILON: f64 = 1e-6;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= EPSILON * a.abs().max(b.abs()).max(1.0)
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

fn check_customers(port: u16, snapshot: &mut Snapshot) -> Result<Vec<Value>, Fail> {
    let path = "/customers?limit=50&offset=0";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    expect_len(arr, PAGE, path)?;
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

    // A target that drops OFFSET still answers the first page correctly, so
    // page 2 is the check that catches it.
    let offset_path = "/customers?limit=50&offset=50";
    let page2 = probe(port, offset_path, snapshot)?;
    let page2 = expect_non_empty(&page2, offset_path)?;
    expect_len(page2, PAGE, offset_path)?;
    let last_of_page1 = id_field(&arr[arr.len() - 1], path)?;
    let first_of_page2 = id_field(&page2[0], offset_path)?;
    if first_of_page2 <= last_of_page1 {
        return Err(Fail::new(
            Code::ParityFail,
            format!(
                "parity {offset_path}: offset page starts at id={first_of_page2}, \
                 which is not past the first page's last id={last_of_page1}"
            ),
        ));
    }

    Ok(arr.clone())
}

fn check_customer_by_id(
    port: u16,
    customers: &[serde_json::Value],
    snapshot: &mut Snapshot,
) -> Result<(), Fail> {
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
    let val = probe(port, &path, snapshot)?;
    let arr = expect_non_empty(&val, &path)?;
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

fn check_employees(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/employees?limit=20&offset=0";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    expect_len(arr, 20, path)?;
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("lastName", is_string),
        ("title", is_string),
        ("birthDate", is_date),
        ("hireDate", is_date),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }
    Ok(())
}

fn check_suppliers(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/suppliers?limit=50&offset=0";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    expect_len(arr, PAGE, path)?;
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

fn check_supplier_by_id(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/supplier-by-id?id=1";
    let val = probe(port, path, snapshot)?;
    expect_non_empty(&val, path)?;
    Ok(())
}

fn check_products(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/products?limit=50&offset=0";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    expect_len(arr, PAGE, path)?;
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

fn check_employee_with_recipient(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/employee-with-recipient?id=2";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("lastName", is_string),
        ("recipientLastName", is_string_or_null),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
        // The joined name must agree with the FK: null name for a null FK,
        // a present name otherwise.
        let has_recipient = row.get("recipientId").is_some_and(|value| !value.is_null());
        let has_name = row
            .get("recipientLastName")
            .is_some_and(|value| !value.is_null());
        if has_recipient != has_name {
            return Err(Fail::new(
                Code::ParityFail,
                format!(
                    "parity {path}: recipientId present={has_recipient} but \
                     recipientLastName present={has_name}"
                ),
            ));
        }
    }
    Ok(())
}

fn check_product_with_supplier(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/product-with-supplier?id=1";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("name", is_string),
        ("supplierId", is_number),
        ("supplier", |v| v.is_object()),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
        let supplier_id = number_field(row, path, "supplierId")?;
        let nested_id = row
            .get("supplier")
            .map(|supplier| number_field(supplier, path, "id"))
            .transpose()?
            .unwrap_or(f64::NAN);
        if !close(supplier_id, nested_id) {
            return Err(Fail::new(
                Code::ParityFail,
                format!(
                    "parity {path}: supplierId={supplier_id} does not match the \
                     nested supplier.id={nested_id}"
                ),
            ));
        }
    }
    Ok(())
}

fn check_orders_with_details(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/orders-with-details?limit=50&offset=0";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    expect_len(arr, PAGE, path)?;
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("shipName", is_string),
        ("productsCount", is_number),
        ("quantitySum", is_number),
        ("totalPrice", is_number),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
    }

    // The aggregates are the part most likely to silently disagree between
    // targets, so cross-check one row against the detail rows it summarises.
    let sample = &arr[0];
    let sample_id = id_field(sample, path)?;
    let detail_path = format!("/order-with-details?id={sample_id}");
    let detail_body = get_json(port, &detail_path)?;
    let detail_rows = expect_non_empty(&detail_body, &detail_path)?;
    let details = detail_rows[0]
        .get("details")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Fail::new(
                Code::ParityFail,
                format!("parity {detail_path}: missing details array"),
            )
        })?;

    let want_count = details.len() as f64;
    let mut want_quantity = 0.0;
    let mut want_total = 0.0;
    for detail in details {
        let quantity = number_field(detail, &detail_path, "quantity")?;
        let unit_price = number_field(detail, &detail_path, "unitPrice")?;
        want_quantity += quantity;
        want_total += quantity * unit_price;
    }

    for (field, want) in [
        ("productsCount", want_count),
        ("quantitySum", want_quantity),
        ("totalPrice", want_total),
    ] {
        let got = number_field(sample, path, field)?;
        if !close(got, want) {
            return Err(Fail::new(
                Code::ParityFail,
                format!(
                    "parity {path}: order {sample_id} reports {field}={got} but its \
                     {} detail rows sum to {want}",
                    details.len()
                ),
            ));
        }
    }
    Ok(())
}

fn check_order_with_details(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/order-with-details?id=1";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("orderDate", is_date),
        ("freight", is_number),
        ("details", is_array),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
        let order_id = id_field(row, path)?;
        let details = row
            .get("details")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        if details.is_empty() {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity {path}: order {order_id} returned zero details"),
            ));
        }
        for detail in details {
            check_fields(
                detail,
                path,
                &[
                    ("unitPrice", is_number),
                    ("quantity", is_number),
                    ("orderId", is_number),
                    ("productId", is_number),
                ],
            )?;
            let detail_order = number_field(detail, path, "orderId")?;
            if !close(detail_order, order_id as f64) {
                return Err(Fail::new(
                    Code::ParityFail,
                    format!(
                        "parity {path}: order {order_id} contains a detail for \
                         orderId={detail_order}"
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn check_order_with_details_and_products(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let path = "/order-with-details-and-products?id=1";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    let fields: &[FieldCheck] = &[
        ("id", is_number),
        ("orderDate", is_date),
        ("details", is_array),
    ];
    for row in arr {
        check_fields(row, path, fields)?;
        let details = row
            .get("details")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        if details.is_empty() {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity {path}: expected at least one detail row"),
            ));
        }
        for detail in details {
            check_fields(
                detail,
                path,
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

fn check_search_customer(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let term = "er";
    let path = "/search-customer?term=er";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    for row in arr.iter().take(10) {
        check_fields(row, path, &[("id", is_number), ("companyName", is_string)])?;
        let company_name = string_field(row, path, "companyName")?;
        if !contains_term(company_name, term) {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity {path}: companyName does not contain term {term:?}"),
            ));
        }
    }
    Ok(())
}

fn check_search_product(port: u16, snapshot: &mut Snapshot) -> Result<(), Fail> {
    let term = "er";
    let path = "/search-product?term=er";
    let val = probe(port, path, snapshot)?;
    let arr = expect_non_empty(&val, path)?;
    for row in arr.iter().take(10) {
        check_fields(row, path, &[("id", is_number), ("name", is_string)])?;
        let name = string_field(row, path, "name")?;
        if !contains_term(name, term) {
            return Err(Fail::new(
                Code::ParityFail,
                format!("parity {path}: name does not contain term {term:?}"),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Canonical bodies and cross-target comparison
// ---------------------------------------------------------------------------

/// Normalize a response body so two targets that agree on the data compare
/// equal despite representation differences.
///
/// - object keys sort (serde_json `Map` is a `BTreeMap` under `preserve_order`
///   being off, but the recursion also normalizes nested values),
/// - dates render as one canonical form: SQLite stores epoch milliseconds and
///   PostgreSQL returns `YYYY-MM-DD`, so date-shaped values become that string,
/// - floats round to `SIGNIFICANT_DIGITS` so `sum()` ordering differences do
///   not read as a data divergence.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, item) in map {
                out.insert(key.clone(), canonicalize_field(key, item));
            }
            Value::Object(out)
        }
        Value::Number(number) => canonicalize_number(number.as_f64()),
        other => other.clone(),
    }
}

fn canonicalize_field(key: &str, value: &Value) -> Value {
    if is_date_field(key) {
        return canonicalize_date(value);
    }
    canonicalize(value)
}

fn is_date_field(key: &str) -> bool {
    matches!(
        key,
        "birthDate" | "hireDate" | "orderDate" | "requiredDate" | "shippedDate"
    )
}

/// Render a date as `YYYY-MM-DD` regardless of the wire representation.
fn canonicalize_date(value: &Value) -> Value {
    match value {
        Value::Number(number) => number
            .as_i64()
            .and_then(epoch_ms_to_date)
            .map_or(Value::Null, Value::String),
        Value::String(text) => Value::String(text.chars().take(10).collect()),
        other => other.clone(),
    }
}

fn epoch_ms_to_date(millis: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|dt| dt.date_naive().format("%Y-%m-%d").to_string())
}

/// Significant digits kept when canonicalizing a float.
///
/// An engine sums an aggregate in whatever order its plan produced the rows,
/// so the low bits of a large `SUM()` legitimately differ between targets that
/// hold identical data — an absolute epsilon would either pass everything at
/// small magnitudes or fail everything at large ones. Twelve significant
/// digits is far more precision than the response contract carries and well
/// short of the ~16 where reassociation shows up.
const SIGNIFICANT_DIGITS: usize = 12;

fn canonicalize_number(value: Option<f64>) -> Value {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return Value::Null;
    };
    let rounded = format!("{value:.*e}", SIGNIFICANT_DIGITS - 1)
        .parse::<f64>()
        .unwrap_or(value);
    serde_json::Number::from_f64(rounded).map_or(Value::Null, Value::Number)
}

/// Routes whose bodies are allowed to differ, keyed by a substring of the
/// target's `sql_variant`.
///
/// A target that documents an intentional deviation still has to answer every
/// other route identically; this list is the whole exemption surface.
/// The markers are deliberately specific phrases rather than loose keywords:
/// several targets mention an "order_id-range detail fetch" while still
/// paginating correctly, and they must stay under the check.
const SNAPSHOT_ALLOWLIST: &[(&str, &[&str])] = &[
    // SpacetimeDB has no OFFSET; the pages come from an id range, so any route
    // that paginates or aggregates over a page can legitimately differ.
    (
        "id-range pagination",
        &[
            "/customers?limit=50&offset=0",
            "/customers?limit=50&offset=50",
            "/employees?limit=20&offset=0",
            "/suppliers?limit=50&offset=0",
            "/products?limit=50&offset=0",
            "/orders-with-details?limit=50&offset=0",
        ],
    ),
    // toasty cannot express the joins, so related rows arrive through extra
    // round trips and the detail-bearing routes may order rows differently.
    (
        "extra round trips",
        &[
            "/order-with-details?id=1",
            "/order-with-details-and-products?id=1",
            "/orders-with-details?limit=50&offset=0",
        ],
    ),
];

fn allowed_divergence(target: &Target, path: &str) -> bool {
    let Some(variant) = target.sql_variant.as_deref() else {
        return false;
    };
    SNAPSHOT_ALLOWLIST
        .iter()
        .any(|(marker, paths)| variant.contains(marker) && paths.contains(&path))
}

/// Compare every target's canonical bodies against the first target's.
///
/// The first target in the run is the reference by construction: the run is
/// only meaningful if all targets answer the same contract, and picking a
/// "correct" one would need a second source of truth.
pub fn compare_snapshots(targets: &[Target], dir: &Path) -> Result<(), Fail> {
    let Some((reference, rest)) = targets.split_first() else {
        return Ok(());
    };
    let expected = read_snapshot(dir, &reference.id)?;

    for target in rest {
        let actual = read_snapshot(dir, &target.id)?;
        for (path, want) in &expected {
            if allowed_divergence(target, path) {
                continue;
            }
            let Some(got) = actual.get(path) else {
                return Err(Fail::new(
                    Code::ParityFail,
                    format!("parity {}: no snapshot for {path}", target.id),
                ));
            };
            if got != want {
                return Err(Fail::new(
                    Code::ParityFail,
                    format!(
                        "parity {}: {path} diverges from {} (reference)",
                        target.id, reference.id
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Path this target's parity run writes its canonical bodies to.
pub fn snapshot_path(dir: &Path, target_id: &str) -> PathBuf {
    dir.join(format!("{target_id}.json"))
}

fn read_snapshot(dir: &Path, target_id: &str) -> Result<Snapshot, Fail> {
    jsonio::read::<Snapshot>(&snapshot_path(dir, target_id), Code::ParityFail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: &str, variant: Option<&str>) -> Target {
        serde_json::from_value(serde_json::json!({
            "version": "v1",
            "id": id,
            "display": { "name": id },
            "lang": "rust",
            "sql_variant": variant,
            "runtime": { "name": "rust", "ver": "1.95.0" },
            "orm": { "name": "drizzle-rs", "ver": "0.1.15" },
            "driver": { "name": "rusqlite", "ver": "0.39.0" },
            "proc": { "mode": "single", "workers": 1 },
            "pool": { "max": 1 },
            "db": { "profile": "sqlite", "hash": format!("sha256:{}", "1".repeat(64)) },
            "wire": { "format": "json" },
            "fair": {
                "workers": 1,
                "pool": 1,
                "db": "sqlite",
                "schema": format!("sha256:{}", "2".repeat(64)),
                "contract": "v1"
            },
            "contract": { "ver": "v1" },
            "parity": { "cmd": ["true"] },
            "load": { "cmd": ["true"] }
        }))
        .expect("target fixture")
    }

    fn write_snapshot(dir: &Path, id: &str, body: Value) {
        let mut snapshot = Snapshot::new();
        snapshot.insert("/customers?limit=50&offset=0".to_string(), body);
        std::fs::write(
            snapshot_path(dir, id),
            serde_json::to_string(&snapshot).expect("serialize"),
        )
        .expect("write snapshot");
    }

    #[test]
    fn dates_and_reassociated_sums_canonicalize_across_dialects() {
        // 2024-03-01T00:00:00Z as epoch millis versus the same day as a
        // PostgreSQL date string, and one `SUM()` summed in two orders.
        let sqlite = serde_json::json!({
            "orderDate": 1_709_251_200_000i64,
            "totalPrice": 130_966_149.16_f64,
        });
        let postgres = serde_json::json!({
            "orderDate": "2024-03-01",
            "totalPrice": 130_966_149.160_000_01_f64,
        });
        assert_eq!(canonicalize(&sqlite), canonicalize(&postgres));
    }

    #[test]
    fn real_value_differences_still_diverge() {
        let a = serde_json::json!({ "totalPrice": 130_966_149.16_f64 });
        let b = serde_json::json!({ "totalPrice": 130_966_149.17_f64 });
        assert_ne!(canonicalize(&a), canonicalize(&b));
    }

    #[test]
    fn divergent_bodies_fail_and_allow_listed_routes_do_not() {
        let dir = tempfile::tempdir().expect("tmp");
        write_snapshot(dir.path(), "reference", serde_json::json!([{ "id": 1 }]));
        write_snapshot(dir.path(), "other", serde_json::json!([{ "id": 2 }]));

        let plain = vec![target("reference", None), target("other", None)];
        let err = compare_snapshots(&plain, dir.path()).expect_err("divergence must fail");
        assert_eq!(err.code, Code::ParityFail);

        let exempt = vec![
            target("reference", None),
            target("other", Some("id-range pagination")),
        ];
        compare_snapshots(&exempt, dir.path()).expect("allow-listed route must pass");
    }

    #[test]
    fn identical_bodies_pass() {
        let dir = tempfile::tempdir().expect("tmp");
        let body = serde_json::json!([{ "id": 1, "orderDate": "2024-03-01" }]);
        write_snapshot(dir.path(), "reference", body.clone());
        write_snapshot(dir.path(), "other", body);

        let targets = vec![target("reference", None), target("other", None)];
        compare_snapshots(&targets, dir.path()).expect("identical snapshots must pass");
    }
}
