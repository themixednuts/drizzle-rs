mod capture;
mod cli;
mod code;
mod load;
mod model;
mod parity;
mod publish;
mod run;
mod schema;

use clap::Parser;
use std::process::ExitCode;

use crate::cli::Cli;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match run::exec(cli).await {
        Ok(code) => code.into(),
        Err(err) => {
            eprintln!("{}", err.msg);
            err.code.into()
        }
    }
}
