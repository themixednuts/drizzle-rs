use crate::cli::Publish;
use crate::code::{Code, Fail};
use crate::model::{RunIndex, RunIndexEntry};
use std::fs;

pub fn run(args: Publish) -> Result<Code, Fail> {
    let manifest_path = args.run.join("manifest.json");
    if !manifest_path.exists() {
        return Err(Fail::new(
            Code::PublishFail,
            format!("manifest not found: {}", manifest_path.display()),
        ));
    }

    #[derive(serde::Deserialize)]
    struct Manifest {
        run_id: String,
        suite: String,
        status: String,
        git: String,
        start: String,
        end: String,
        targets: Vec<String>,
        runner: ManifestRunner,
    }
    #[derive(serde::Deserialize)]
    struct ManifestRunner {
        class: String,
    }

    let body = fs::read_to_string(&manifest_path).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!("failed to read {}: {err}", manifest_path.display()),
        )
    })?;
    let manifest: Manifest = serde_json::from_str(&body).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!("invalid manifest {}: {err}", manifest_path.display()),
        )
    })?;

    let mut index = if args.index.exists() {
        let idx_body = fs::read_to_string(&args.index).map_err(|err| {
            Fail::new(
                Code::PublishFail,
                format!("failed to read index {}: {err}", args.index.display()),
            )
        })?;
        serde_json::from_str::<RunIndex>(&idx_body).map_err(|err| {
            Fail::new(
                Code::PublishFail,
                format!("invalid index {}: {err}", args.index.display()),
            )
        })?
    } else {
        RunIndex {
            version: "v1".to_string(),
            runs: Vec::new(),
        }
    };

    // Deduplicate: remove any existing entry with the same run_id
    index.runs.retain(|r| r.run_id != manifest.run_id);

    index.runs.push(RunIndexEntry {
        run_id: manifest.run_id.clone(),
        suite: manifest.suite,
        status: manifest.status,
        class: manifest.runner.class,
        git: manifest.git,
        start: manifest.start,
        end: manifest.end,
        targets: manifest.targets,
    });

    let out_path = args.out_index.as_ref().unwrap_or(&args.index);
    let out_body = serde_json::to_string_pretty(&index).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!("failed to serialize index: {err}"),
        )
    })?;
    fs::write(out_path, &out_body).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!("failed to write index {}: {err}", out_path.display()),
        )
    })?;

    validate_index(out_path)?;

    println!("published run_id={}", manifest.run_id);
    Ok(Code::Success)
}

fn validate_index(path: &std::path::Path) -> Result<(), Fail> {
    let body = fs::read_to_string(path).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!("failed to read index for validation: {err}"),
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|err| Fail::new(Code::PublishFail, format!("invalid index json: {err}")))?;

    let schema_path = workspace_root()
        .join("docs")
        .join("benchmark-spec")
        .join("jsonschema")
        .join("run-index.v1.schema.json");
    let schema_body = fs::read_to_string(&schema_path).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!(
                "failed to read index schema {}: {err}",
                schema_path.display()
            ),
        )
    })?;
    let schema: serde_json::Value = serde_json::from_str(&schema_body)
        .map_err(|err| Fail::new(Code::PublishFail, format!("invalid index schema: {err}")))?;

    let validator = jsonschema::validator_for(&schema).map_err(|err| {
        Fail::new(
            Code::PublishFail,
            format!("index schema compile failed: {err}"),
        )
    })?;

    let errors: Vec<String> = validator
        .iter_errors(&value)
        .take(4)
        .map(|err| {
            let at = err.instance_path().to_string();
            if at.is_empty() {
                err.to_string()
            } else {
                format!("{at}: {err}")
            }
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Fail::new(
            Code::PublishFail,
            format!("index schema validation failed: {}", errors.join("; ")),
        ))
    }
}

fn workspace_root() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    dir.parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| dir.to_path_buf())
}
