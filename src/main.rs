use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cache;
mod config;
mod init;
mod lang;
mod log;
mod mcp;
mod model;
mod scan;
mod walker;

#[derive(Parser)]
#[command(
    name = "code-seek",
    version,
    about = "Explore the functional structure of a repository"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        force: bool,
    },
    Mcp,
    Scan {
        path: PathBuf,
        #[arg(long, value_delimiter = ',', value_name = "LANG")]
        lang: Vec<String>,
        #[arg(long, default_value = "tree", value_name = "FORMAT")]
        format: String,
        #[arg(long = "match", value_name = "PATTERN")]
        filter_match: Option<String>,
        #[arg(long, value_name = "N")]
        max_depth: Option<usize>,
        #[arg(long, value_delimiter = ',', value_name = "DIR")]
        ignore: Vec<String>,
        #[arg(long, default_value = "all", value_name = "INFO")]
        info: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { force } => init::run(force)?,
        Command::Mcp => mcp::run()?,
        Command::Scan { path, lang, format, filter_match, max_depth, ignore, info } => {
            let info_lower = info.to_lowercase();
            if !["all", "no-tests", "tests-only"].contains(&info_lower.as_str()) {
                return Err(format!("invalid --info value '{info}'. Valid: all, no-tests, tests-only").into());
            }
            scan::run(&path, &lang, &format, filter_match.as_deref().unwrap_or(""), max_depth, &ignore, &info_lower)?;
        }
    }
    Ok(())
}
