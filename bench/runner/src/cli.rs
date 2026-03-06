use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "bench-runner")]
#[command(about = "Benchmark runner")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    Run(Run),
    Capture(Capture),
    Load(Load),
    Parity(Parity),
    Validate(Validate),
    Publish(Publish),
}

#[derive(Debug, Clone, Args)]
pub struct Run {
    #[arg(long, value_enum)]
    pub suite: Suite,

    #[arg(long)]
    pub workload: PathBuf,

    #[arg(long)]
    pub targets: PathBuf,

    #[arg(long)]
    pub requests: PathBuf,

    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, value_enum, default_value_t = Class::Publish)]
    pub class: Class,

    #[arg(long)]
    pub trials: Option<u32>,

    #[arg(long)]
    pub baseline: Option<String>,

    #[arg(long)]
    pub publish: bool,

    #[arg(long)]
    pub seed: Option<u64>,

    #[arg(long)]
    pub timeout_s: Option<u64>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
#[command(trailing_var_arg = true)]
pub struct Capture {
    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 1000)]
    pub ms: u64,

    #[arg(long)]
    pub label: Option<String>,

    #[arg(required = true, num_args = 1..)]
    pub cmd: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct Load {
    #[arg(long)]
    pub out: Option<PathBuf>,

    #[arg(long)]
    pub target: Option<String>,

    #[arg(long)]
    pub trial: Option<u32>,

    #[arg(long)]
    pub seed: Option<u64>,

    #[arg(long, value_enum)]
    pub suite: Option<Suite>,

    #[arg(long)]
    pub workload: Option<PathBuf>,

    #[arg(long)]
    pub requests: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct Parity {
    #[arg(long)]
    pub target: Option<String>,

    #[arg(long)]
    pub seed: Option<u64>,

    #[arg(long)]
    pub trial: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct Validate {
    #[arg(long)]
    pub run: PathBuf,

    #[arg(long)]
    pub workload: Option<PathBuf>,

    #[arg(long)]
    pub targets: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct Publish {
    /// Path to the completed run directory
    #[arg(long)]
    pub run: PathBuf,

    /// Path to existing index.json (will be created if missing)
    #[arg(long)]
    pub index: PathBuf,

    /// Path to write updated index.json (defaults to --index path)
    #[arg(long)]
    pub out_index: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum Suite {
    ThroughputHttp,
    MvccContention,
}

impl Suite {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ThroughputHttp => "throughput-http",
            Self::MvccContention => "mvcc-contention",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Small,
    Publish,
}

impl Class {
    pub const fn default_trials(self) -> u32 {
        match self {
            Self::Small => 1,
            Self::Publish => 5,
        }
    }
}
