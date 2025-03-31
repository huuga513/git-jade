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
    /// Manage branches
    Branch {
        /// Name of the branch
        #[clap(value_name = "NAME", required = true)]
        name: String,

        /// Delete the branch
        #[clap(short = 'd', long = "delete")]
        delete: bool,
    },
    /// Switch branches or restore working tree files
    Checkout {
        /// Target branch/commit to checkout
        #[clap(value_name = "TARGET", required = true)]
        target: String,
    },
    /// Merge another branch into current branch
    Merge {
        /// Branch name to merge
        #[clap(value_name = "BRANCH", required = true)]
        branch: String,
    },
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
        Command::Branch { name, delete } => {
            if delete {
                println!("Deleting branch '{}'", name);
            } else {
                println!("Creating branch '{}'", name);
            }
        }
        Command::Checkout { target } => {
            println!("Checking out to: {}", target);
        }
        Command::Merge { branch } => {
            println!("Merging branch: {}", branch);
        }
    }
}