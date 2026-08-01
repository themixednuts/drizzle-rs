mod capture;
mod cli;
mod clock;
mod code;
mod jsonio;
mod load;
mod model;
mod parity;
mod proc;
mod publish;
mod run;
mod schema;
mod stats;
mod workload_terms;

use clap::Parser;
use std::process::ExitCode;

use crate::cli::{Cli, Cmd};

fn main() -> ExitCode {
    let cli = Cli::parse();

    // The runtime is built by hand rather than with `#[tokio::main]` because the
    // two roles need opposite sizing:
    //
    // - `serve` *is* the system under test. It must use exactly the worker count
    //   the target spec declares (`BENCH_WORKERS`, default 1), otherwise a Rust
    //   target quietly gets every core while the Bun targets it is ranked
    //   against get one, and `fair.workers: 1` is a lie.
    // - every other subcommand is tooling (load generation, aggregation) and
    //   should use the whole machine.
    let worker_threads = match &cli.cmd {
        Cmd::Serve(_) => load::configured_workers(),
        _ => std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("failed to build tokio runtime: {err}");
            return ExitCode::from(code::Code::RunFail as u8);
        }
    };

    runtime.block_on(async {
        match run::exec(cli).await {
            Ok(code) => code.into(),
            Err(err) => {
                eprintln!("{}", err.msg);
                err.code.into()
            }
        }
    })
}
