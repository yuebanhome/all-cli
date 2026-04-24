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

    let cfg = config::load()?;
    let effective = api::EffectiveCfg {
        base_url: cfg.base_url,
        api_key: cfg.api_key,
        model: args.model.clone()
            .or(cfg.defaults.model)
            .unwrap_or_else(|| "gpt-image-2".into()),
        size: cfg.defaults.size.unwrap_or_else(|| "auto".into()),
        quality: args.quality.clone()
            .or(cfg.defaults.quality)
            .unwrap_or_else(|| "auto".into()),
        prompt: args.prompt.clone().expect("validated above"),
        image: args.image.clone(),
        mask: args.mask.clone(),
    };

    let log = logging::Logger::new(args.quiet);
    let client = client::build_client()?;
    let resp = if effective.image.is_some() {
        anyhow::bail!("input error: edit endpoint not implemented yet");
    } else {
        api::generate(&client, &effective, &log)?
    };

    let out = args.output.clone().expect("validated above");
    output::save_first_image(&resp, &out)?;
    println!("Saved: {}", out.display());
    Ok(())
}
