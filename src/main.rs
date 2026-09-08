use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod config;
mod init;
mod lang;
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
    Scan {
        path: PathBuf,
        #[arg(long, value_delimiter = ',', value_name = "LANG")]
        lang: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init { force } => init::run(force),
        Command::Scan { path, lang } => scan::run(&path, &lang),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
