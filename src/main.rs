use clap::{Parser, Subcommand};
use git_rs::{Repository, repo};
use std::{env::current_dir, path::{Path, PathBuf}};

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

fn find_repo_dir() -> PathBuf {
    let repo_dir = current_dir().unwrap();
    repo_dir
}
fn open_repo(repo_dir: &Path) -> Repository {
    let repo = match Repository::open(&repo_dir) {
        Ok(repo) => repo,
        Err(why) => {
            println!("{why}");
            std::process::exit(-1);
        }
    };
    repo
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Commit { message } => {
            println!("Commit message: {}", message);
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.commit(message);
        }
        Command::Add { paths } => {
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.add(&paths);
        }
        Command::Init => {
            let current_dir = current_dir().unwrap();
            let _ = match Repository::init(&current_dir) {
                Ok(repo) => repo,
                Err(why) => {
                    println!("{why}");
                    std::process::exit(-1);
                }
            };
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
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.checkout(&target);

        }
        Command::Merge { branch } => {
            println!("Merging branch: {}", branch);
        }
    }
}
