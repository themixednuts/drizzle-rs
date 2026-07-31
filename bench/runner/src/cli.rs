use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "bench-runner", version)]
#[command(about = "Benchmark runner", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Execute a full benchmark run and write the artifact set.
    Run(Run),
    /// Record host telemetry while an arbitrary command runs.
    Capture(Capture),
    /// Serve one built-in target until killed (used as a target `server.cmd`).
    Serve(Serve),
    /// Drive load against one target for a single trial.
    Load(Load),
    /// Check one target's responses against the route contract.
    Parity(Parity),
    /// Seed the PostgreSQL database an external target will use.
    SeedPostgres(SeedPostgres),
    /// Validate a completed run (and optionally its inputs) against the schemas.
    Validate(Validate),
    /// Append a completed run to the run index.
    Publish(Publish),
}

#[derive(Debug, Clone, Args)]
pub struct Run {
    /// Benchmark suite; must match `workload.suite`.
    #[arg(long, value_enum)]
    pub suite: Suite,

    /// Workload spec (workload.v1 schema).
    #[arg(long)]
    pub workload: PathBuf,

    /// Target list (array of target.v1 descriptors).
    #[arg(long)]
    pub targets: PathBuf,

    /// Request list; an empty array generates the deterministic default mix.
    #[arg(long)]
    pub requests: PathBuf,

    /// Output root; artifacts land in `<out>/runs/<run_id>/`.
    #[arg(long)]
    pub out: PathBuf,

    /// Groups runs from different machines into one logical comparison.
    /// Defaults to this run's id.
    #[arg(long)]
    pub cohort_id: Option<String>,

    /// Run class. Controls trial count, seed handling, and gate severity.
    #[arg(long, value_enum, default_value_t = Class::Publish)]
    pub class: Class,

    /// Override the class default trial count.
    #[arg(long)]
    pub trials: Option<u32>,

    /// Regression baseline: a run id, or `auto` for the newest compatible run.
    #[arg(long)]
    pub baseline: Option<String>,

    /// Append this run to `<out>/index.json` when it completes.
    #[arg(long)]
    pub publish: bool,

    /// Dataset seed. Honoured for `--class small` only.
    #[arg(long)]
    pub seed: Option<u64>,

    /// Default per-invocation timeout for target parity/warmup/load commands.
    #[arg(long, default_value_t = DEFAULT_EXEC_TIMEOUT_S)]
    pub timeout_s: u64,

    /// Emit JSONL progress events on stdout.
    #[arg(long)]
    pub json: bool,
}

/// Wall-clock budget for a single target hook when the spec does not set one.
/// Without a default, a wedged target hangs CI instead of failing it.
pub const DEFAULT_EXEC_TIMEOUT_S: u64 = 1800;

#[derive(Debug, Clone, Args)]
#[command(trailing_var_arg = true)]
pub struct Capture {
    /// Directory to write `host.json`, `samples.jsonl`, and `summary.json` into.
    #[arg(long)]
    pub out: PathBuf,

    /// Sampling interval in milliseconds.
    #[arg(long, default_value_t = 1000)]
    pub ms: u64,

    /// Label recorded in `host.json`.
    #[arg(long)]
    pub label: Option<String>,

    /// Command to run, after `--`.
    #[arg(required = true, num_args = 1..)]
    pub cmd: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct Load {
    /// Timeseries output path. Falls back to `BENCH_TIMESERIES_OUT`.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Target id. Falls back to `BENCH_TARGET_ID`.
    #[arg(long)]
    pub target: Option<String>,

    /// Zero-based trial index; decorrelates the request order across trials.
    /// Falls back to `BENCH_TRIAL`.
    #[arg(long)]
    pub trial: Option<u32>,

    /// Dataset seed. Falls back to `BENCH_SEED`.
    #[arg(long)]
    pub seed: Option<u64>,

    /// Benchmark suite. Falls back to `BENCH_SUITE`.
    #[arg(long, value_enum)]
    pub suite: Option<Suite>,

    /// Workload spec. Falls back to `BENCH_WORKLOAD_FILE`.
    #[arg(long)]
    pub workload: Option<PathBuf>,

    /// Materialized request list. Falls back to `BENCH_REQUESTS_FILE`.
    #[arg(long)]
    pub requests: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct Serve {
    /// Target id. Falls back to `BENCH_TARGET_ID`.
    #[arg(long)]
    pub target: Option<String>,

    /// Dataset seed. Falls back to `BENCH_SEED`.
    #[arg(long)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Args)]
pub struct Parity {
    /// Target id. Falls back to `BENCH_TARGET_ID`.
    #[arg(long)]
    pub target: Option<String>,

    /// Dataset seed. Falls back to `BENCH_SEED`, then 42.
    #[arg(long)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Args)]
pub struct SeedPostgres {
    /// Dataset seed. Falls back to `BENCH_SEED`, then 42.
    #[arg(long)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Args)]
pub struct Validate {
    /// Completed run directory.
    #[arg(long)]
    pub run: PathBuf,

    /// Also validate this workload spec.
    #[arg(long)]
    pub workload: Option<PathBuf>,

    /// Also validate this target list.
    #[arg(long)]
    pub targets: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct Publish {
    /// Completed run directory.
    #[arg(long)]
    pub run: PathBuf,

    /// Existing index.json; created when missing.
    #[arg(long)]
    pub index: PathBuf,

    /// Where to write the updated index. Defaults to `--index`.
    #[arg(long)]
    pub out_index: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum Suite {
    ThroughputHttp,
}

impl Suite {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ThroughputHttp => "throughput-http",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// Local and PR smoke runs. Honours `--seed`; gate failures are reported,
    /// not fatal.
    Small,
    /// Full matrix, not published. Uses the workload seed.
    Full,
    /// Publish-grade. Uses the workload seed; any gate failure exits 9.
    Publish,
}

impl Class {
    pub const fn default_trials(self) -> u32 {
        match self {
            Self::Small => 3,
            Self::Full => 5,
            Self::Publish => 5,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Full => "full",
            Self::Publish => "publish",
        }
    }

    /// Whether the workload seed is authoritative regardless of `--seed`.
    pub const fn pins_workload_seed(self) -> bool {
        matches!(self, Self::Full | Self::Publish)
    }
}
