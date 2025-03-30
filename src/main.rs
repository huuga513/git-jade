use clap::{Parser, Subcommand};
use git_rs::Repository;
use std::path::Path;

#[derive(Parser)]
#[command(name = "rugit")]
#[command(version = "1.0")]
#[command(about = "A simple Git client written in Rust", long_about = None)]
#[derive(Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}
#[derive(Debug, Subcommand)]
enum Command {
    /// Commit changes to repository
    Commit {
        /// Commit message
        #[clap(short = 'm', long = "message", required = true)]
        message: String,
    },
    
    /// Add files to staging area
    Add {
        /// Paths to files/directories to add
        #[clap(required = true)]
        paths: Vec<String>,
    },
    /// Initialize a new repository
    Init,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Commit { message } => {
            println!("Commit message: {}", message);
            // Actual commit logic here
        },
        Command::Add { paths } => {
            println!("Adding paths: {:?}", paths);
            // Actual add logic here
        }
        Command::Init => {
            println!("Initializing empty repository");
        }
    }
}