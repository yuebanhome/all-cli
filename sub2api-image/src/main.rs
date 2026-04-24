mod api;
mod cli;
mod client;
mod config;
mod errors;
mod logging;
mod output;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = cli::Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            for cause in e.chain().skip(1) {
                eprintln!("  caused by: {}", cause);
            }
            ExitCode::from(errors::exit_code_from(&e))
        }
    }
}

fn run(args: &cli::Args) -> anyhow::Result<()> {
    args.validate()?;
    if args.init {
        let path = config::write_template()?;
        println!("Wrote config template to {}", path.display());
        println!("Edit it with your base_url and api_key, then re-run.");
        return Ok(());
    }
    // 后续 task 里补 API 调用逻辑
    anyhow::bail!("input error: API path not implemented yet (placeholder)");
}
