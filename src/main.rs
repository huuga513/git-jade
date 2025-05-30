use clap::{Parser, Subcommand};
use rust_git::Repository;
use std::{env::current_dir, path::{Path, PathBuf}};

#[derive(Parser)]
#[command(name = "rust-git")]
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
    /// Print the status
    Status,
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

        /// Create a branch
        #[clap(short = 'b')]
        create: bool,
    },
    /// Merge another branch into current branch
    Merge {
        /// Branch name to merge
        #[clap(value_name = "BRANCH", required = true)]
        branch: String,
    },
    /// Remove a file
    Rm {
        /// Paths to files/directories to remove
        #[clap(required = true)]
        paths: Vec<String>,
    }
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
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.commit(message);
        }
        Command::Add { paths } => {
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.add(&paths);
        }
        Command::Rm { paths } => {
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.rm(&paths);
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
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            if delete {
                repo.rm_branch(name);
            } else {
                repo.branch(name);
            }
        }
        Command::Checkout { target , create} => {
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            if create {
                repo.branch(&target);
            }
            repo.checkout(&target);

        }
        Command::Merge { branch } => {
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.merge(&branch); 
        }
        Command::Status => {
            let repo_dir = find_repo_dir();
            let repo = open_repo(&repo_dir);
            repo.status();
        }
    }
}
