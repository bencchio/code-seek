use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod init;
mod scan;
mod walker;

#[derive(Parser)]
#[command(name = "codexa", version, about = "Explore the functional structure of a repository")]
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
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init { force } => init::run(force),
        Command::Scan { path } => scan::run(&path),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
