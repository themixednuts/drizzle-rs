//! JSON Schema validation for every persisted artifact.
//!
//! Schemas are compiled into the binary with `include_str!`. Resolving them
//! relative to `CARGO_MANIFEST_DIR` at runtime only worked when the binary was
//! run from inside the source tree — the release binary that CI ships would
//! have failed to find its own contracts.

use crate::code::{Code, Fail};
use crate::jsonio;
use serde_json::Value;
use std::fs;
use std::path::Path;

const WORKLOAD_SCHEMA: &str =
    include_str!("../../../docs/benchmark-spec/jsonschema/workload.v1.schema.json");
const TARGET_SCHEMA: &str =
    include_str!("../../../docs/benchmark-spec/jsonschema/target.v1.schema.json");
const MANIFEST_SCHEMA: &str =
    include_str!("../../../docs/benchmark-spec/jsonschema/run-manifest.v1.schema.json");
const SUMMARY_SCHEMA: &str =
    include_str!("../../../docs/benchmark-spec/jsonschema/summary.v1.schema.json");
const TIMESERIES_SCHEMA: &str =
    include_str!("../../../docs/benchmark-spec/jsonschema/timeseries.v1.schema.json");
const RUN_INDEX_SCHEMA: &str =
    include_str!("../../../docs/benchmark-spec/jsonschema/run-index.v1.schema.json");

pub fn validate_workload(path: &Path) -> Result<(), Fail> {
    validate_file(path, WORKLOAD_SCHEMA, "workload", Code::InvalidInput)
}

pub fn validate_targets(path: &Path) -> Result<(), Fail> {
    let value: Value = jsonio::read(path, Code::InvalidInput)?;
    let items = value.as_array().ok_or_else(|| {
        Fail::new(
            Code::InvalidInput,
            format!("targets file must be a JSON array: {}", path.display()),
        )
    })?;
    if items.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("targets list must not be empty: {}", path.display()),
        ));
    }

    let validator = compile(TARGET_SCHEMA, "target", Code::InvalidInput)?;
    for (idx, item) in items.iter().enumerate() {
        validate_value(
            &validator,
            item,
            &format!("targets[{idx}]"),
            path,
            Code::InvalidInput,
        )?;
    }
    Ok(())
}

pub fn validate_manifest(path: &Path) -> Result<(), Fail> {
    validate_file(path, MANIFEST_SCHEMA, "manifest", Code::AggregateFail)
}

pub fn validate_summary(path: &Path) -> Result<(), Fail> {
    validate_file(path, SUMMARY_SCHEMA, "summary", Code::AggregateFail)
}

pub fn validate_timeseries(path: &Path) -> Result<(), Fail> {
    validate_file(path, TIMESERIES_SCHEMA, "timeseries", Code::AggregateFail)
}

pub fn validate_index(path: &Path) -> Result<(), Fail> {
    validate_file(path, RUN_INDEX_SCHEMA, "index", Code::PublishFail)
}

pub fn validate_run(run: &Path) -> Result<(), Fail> {
    validate_manifest(&run.join("manifest.json"))?;

    let targets = run.join("targets");
    let mut seen = 0_usize;
    for entry in fs::read_dir(&targets).map_err(|err| {
        Fail::new(
            Code::InvalidInput,
            format!("failed to read {}: {err}", targets.display()),
        )
    })? {
        let entry = entry.map_err(|err| {
            Fail::new(
                Code::InvalidInput,
                format!("target dir entry failed: {err}"),
            )
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        validate_summary(&path.join("summary.json"))?;
        validate_timeseries(&path.join("timeseries.json"))?;
        seen += 1;
    }

    if seen == 0 {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("run has no target artifacts: {}", run.display()),
        ));
    }
    Ok(())
}

fn validate_file(path: &Path, schema: &str, label: &str, code: Code) -> Result<(), Fail> {
    let value: Value = jsonio::read(path, code)?;
    let validator = compile(schema, label, code)?;
    validate_value(&validator, &value, label, path, code)
}

fn compile(schema: &str, label: &str, code: Code) -> Result<jsonschema::Validator, Fail> {
    let schema: Value = serde_json::from_str(schema)
        .map_err(|err| Fail::new(code, format!("{label} schema is not valid json: {err}")))?;
    jsonschema::validator_for(&schema)
        .map_err(|err| Fail::new(code, format!("{label} schema compile failed: {err}")))
}

fn validate_value(
    validator: &jsonschema::Validator,
    value: &Value,
    label: &str,
    path: &Path,
    code: Code,
) -> Result<(), Fail> {
    let errs = validator
        .iter_errors(value)
        .take(4)
        .map(|err| {
            let at = err.instance_path().to_string();
            if at.is_empty() {
                err.to_string()
            } else {
                format!("{at}: {err}")
            }
        })
        .collect::<Vec<_>>();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(Fail::new(
            code,
            format!(
                "{label} schema validation failed for {}: {}",
                path.display(),
                errs.join("; ")
            ),
        ))
    }
}
