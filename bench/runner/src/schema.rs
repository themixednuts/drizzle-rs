use crate::code::{Code, Fail};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn validate_workload(path: &Path) -> Result<(), Fail> {
    validate_file(
        path,
        "workload.v1.schema.json",
        "workload",
        Code::InvalidInput,
    )
}

pub fn validate_targets(path: &Path) -> Result<(), Fail> {
    let value = read_json(path, Code::InvalidInput)?;
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

    let schema = read_json(&schema_path("target.v1.schema.json"), Code::InvalidInput)?;
    let validator = jsonschema::validator_for(&schema).map_err(|err| {
        Fail::new(
            Code::InvalidInput,
            format!("target schema compile failed: {err}"),
        )
    })?;

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
    validate_file(
        path,
        "run-manifest.v1.schema.json",
        "manifest",
        Code::AggregateFail,
    )
}

pub fn validate_summary(path: &Path) -> Result<(), Fail> {
    validate_file(
        path,
        "summary.v1.schema.json",
        "summary",
        Code::AggregateFail,
    )
}

pub fn validate_timeseries(path: &Path) -> Result<(), Fail> {
    validate_file(
        path,
        "timeseries.v1.schema.json",
        "timeseries",
        Code::AggregateFail,
    )
}

pub fn validate_run(run: &Path) -> Result<(), Fail> {
    let manifest = run.join("manifest.json");
    validate_manifest(&manifest)?;

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

fn validate_file(path: &Path, schema_name: &str, label: &str, code: Code) -> Result<(), Fail> {
    let value = read_json(path, code)?;
    let schema = read_json(&schema_path(schema_name), code)?;
    let validator = jsonschema::validator_for(&schema)
        .map_err(|err| Fail::new(code, format!("{label} schema compile failed: {err}")))?;
    validate_value(&validator, &value, label, path, code)
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

fn read_json(path: &Path, code: Code) -> Result<Value, Fail> {
    let body = fs::read_to_string(path)
        .map_err(|err| Fail::new(code, format!("failed to read {}: {err}", path.display())))?;
    serde_json::from_str(&body)
        .map_err(|err| Fail::new(code, format!("invalid json {}: {err}", path.display())))
}

fn schema_path(name: &str) -> PathBuf {
    workspace_root()
        .join("docs")
        .join("benchmark-spec")
        .join("jsonschema")
        .join(name)
}

fn workspace_root() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    dir.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.to_path_buf())
}
