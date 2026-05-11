use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "playctl",
    version,
    about = "Local HTTP playground server controller"
)]
pub struct Cli {
    /// Override project root detection
    #[arg(long, value_name = "PATH", global = true)]
    pub root: Option<PathBuf>,

    /// Base port to try (increments up to +15 on EADDRINUSE)
    #[arg(long, default_value_t = 4747, global = true)]
    pub port: u16,

    /// Print only the URL on success, only errors on failure
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Emit machine-readable JSON output
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Idempotent start; prints URL
    Start,
    /// Graceful shutdown
    Stop,
    /// Status + URL + count
    Status,
    /// List playgrounds
    List,
    /// Scaffold a new playground from a template
    New {
        slug: String,
        #[arg(long, default_value = "design-playground")]
        template: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Rescan .playgrounds/, rebuild index.json
    Reindex,
    /// Open project URL (or specific slug) in default browser
    Open { slug: Option<String> },
    /// Print embedded template markdown to stdout
    PrintTemplate { name: String },
    /// (Internal) actually run the axum server in foreground; used by daemon child
    #[command(hide = true, name = "serve-foreground")]
    ServeForeground {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        port: u16,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_start() {
        let c = Cli::try_parse_from(["playctl", "start"]).unwrap();
        assert!(matches!(c.cmd, Cmd::Start));
        assert_eq!(c.port, 4747);
    }

    #[test]
    fn parses_new_with_flags() {
        let c = Cli::try_parse_from([
            "playctl",
            "new",
            "card-tuner",
            "--template",
            "design-playground",
            "--title",
            "Card Tuner",
        ])
        .unwrap();
        match c.cmd {
            Cmd::New {
                slug,
                template,
                title,
                description,
            } => {
                assert_eq!(slug, "card-tuner");
                assert_eq!(template, "design-playground");
                assert_eq!(title.as_deref(), Some("Card Tuner"));
                assert_eq!(description, None);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parses_print_template() {
        let c = Cli::try_parse_from(["playctl", "print-template", "design-playground"]).unwrap();
        assert!(matches!(c.cmd, Cmd::PrintTemplate { .. }));
    }

    #[test]
    fn global_flags_after_subcommand() {
        let c = Cli::try_parse_from(["playctl", "--port", "5000", "start"]).unwrap();
        assert_eq!(c.port, 5000);
    }
}
