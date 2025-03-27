use clap::{Parser, Subcommand};
use git_rs::Repository;
use std::path::Path;

#[derive(Parser)]
#[command(name = "rugit")]
#[command(version = "1.0")]
#[command(about = "A simple Git client written in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new repository
    Init ,
    
    /// Add files to staging area
    Add {
        path: String,
    },
    
    /// Remove files from repository
    Rm {
        path: String,
    },
    
    /// Commit changes
    Commit {
        message: String,
    },
    
    /// Branch operations
    Branch {
        name: Option<String>,
        #[arg(short, long)]
        delete: bool,
    },
    
    /// Checkout branch/commit
    Checkout {
        target: String,
    },
    
    /// Merge branches
    Merge {
        branch: String,
    },
    
    /// Fetch from remote
    Fetch,
    
    /// Pull from remote
    Pull,
    
    /// Push to remote
    Push,
}

fn main() {
    let args = Cli::parse();
    
    match args.command {
        Commands::Init => todo!("Init"),
        Commands::Add { path } => todo!("Add"),
        Commands::Rm { path } => todo!("Rm"),
        Commands::Commit { message } => todo!("Commit"),
        Commands::Branch { name, delete } => todo!("Branch"),
        Commands::Checkout { target } => todo!("Checkout"),
        Commands::Merge { branch } => todo!("Merge"),
        Commands::Fetch => todo!("Fetch"),
        Commands::Pull => todo!("pull"),
        Commands::Push => todo!("push"),
    }
}

// // Core Git operations implementation
// fn handle_init(path: String) {
    // Repository::init(Path::new(&path))
        // .unwrap_or_else(|e| panic!("Failed to initialize repository: {}", e));
    // println!("Initialized empty repository at {}", path);
// }

// fn handle_add(files: Vec<String>) {
    // let repo = Repository::open(".").expect("Not a git repository");
    // let mut index = repo.index().expect("Cannot get repository index");
    
    // files.iter().for_each(|file| {
        // index.add_path(Path::new(file))
            // .unwrap_or_else(|e| panic!("Failed to add file {}: {}", file, e));
    // });
    
    // index.write().expect("Failed to write index");
    // println!("Added {} files to staging area", files.len());
// }

// fn handle_commit(message: &str) {
    // let repo = Repository::open(".").expect("Not a git repository");
    // let signature = repo.signature().expect("Cannot get user signature");
    // let mut index = repo.index().expect("Cannot get repository index");
    // let oid = index.write_tree().expect("Cannot write tree");
    // let tree = repo.find_tree(oid).expect("Cannot find tree");
    
    // let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    
    // let parents = match parent_commit {
        // Some(commit) => vec![&commit],
        // None => vec![],
    // };
    
    // repo.commit(
        // Some("HEAD"),
        // &signature,
        // &signature,
        // message,
        // &tree,
        // &parents,
    // ).expect("Failed to create commit");
    
    // println!("Created commit with message: {}", message);
// }

// fn handle_branch(name: Option<String>, delete: bool) {
    // let repo = Repository::open(".").expect("Not a git repository");
    
    // match name {
        // Some(name) if delete => {
            // let branch = repo.find_branch(&name, BranchType::Local)
                // .expect("Branch not found");
            // branch.delete().expect("Failed to delete branch");
            // println!("Deleted branch {}", name);
        // },
        // Some(name) => {
            // let head = repo.head().expect("Cannot get HEAD");
            // let commit = head.peel_to_commit().expect("Cannot get commit");
            // repo.branch(&name, &commit, false).expect("Failed to create branch");
            // println!("Created branch {}", name);
        // },
        // None => {
            // // List branches
            // repo.branches(Some(BranchType::Local))
                // .expect("Cannot list branches")
                // .for_each(|b| {
                    // let (branch, _) = b.expect("Error reading branch");
                    // println!("{}", branch.name().unwrap().unwrap());
                // });
        // }
    // }
// }

// // Error handling helper
// fn handle_git_error(e: Error) -> ! {
    // eprintln!("Git operation failed: {}", e);
    // std::process::exit(1);
// }