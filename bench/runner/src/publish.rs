use crate::cli::Publish;
use crate::code::{Code, Fail};
use crate::jsonio;
use crate::model::{ManifestSummary, RunIndex, RunIndexEntry};

/// Append a completed run to the run index, replacing any entry with the same
/// `run_id` so re-publishing is idempotent.
pub fn run(args: Publish) -> Result<Code, Fail> {
    let manifest_path = args.run.join("manifest.json");
    if !manifest_path.exists() {
        return Err(Fail::new(
            Code::PublishFail,
            format!("manifest not found: {}", manifest_path.display()),
        ));
    }
    let manifest: ManifestSummary = jsonio::read(&manifest_path, Code::PublishFail)?;

    let mut index = if args.index.exists() {
        jsonio::read::<RunIndex>(&args.index, Code::PublishFail)?
    } else {
        RunIndex {
            version: "v1".to_string(),
            runs: Vec::new(),
        }
    };

    index.runs.retain(|run| run.run_id != manifest.run_id);
    index.runs.push(RunIndexEntry {
        run_id: manifest.run_id.clone(),
        cohort_id: manifest.cohort_id,
        name: manifest.name,
        suite: manifest.suite,
        status: manifest.status,
        class: manifest.runner.class,
        git: manifest.git,
        start: manifest.start,
        end: manifest.end,
        targets: manifest.targets,
    });

    let out_path = args.out_index.as_ref().unwrap_or(&args.index);
    jsonio::write(out_path, &index, Code::PublishFail)?;
    crate::schema::validate_index(out_path)?;

    println!("published run_id={}", manifest.run_id);
    Ok(Code::Success)
}
