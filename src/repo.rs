use core::error;
use std::error::Error;
use std::{env, fs};
use std::path::{Path, PathBuf};
use super::object::ObjectDB;
const OBJECTS_DIR: &str = "objects";
const REFS_DIR: &str = "refs";
const HEAD_FILE: &str = "HEAD";
const GIT_DIR: &str = ".git";

pub struct Repository {
    dir: PathBuf, // Path to the repository directory.
    git_dir: PathBuf, // Path to the git directory ({dir}/{GIT_DIR}).
    work_dir: PathBuf, // Path to the current working directory.
    obj_db: ObjectDB,
}

impl Repository {
    pub fn is_vaild_git_dir(path: &Path) -> bool {
        let git_dir = path;

        if !git_dir.is_dir() {
            return false;
        }

        // 检查HEAD文件是否存在且为文件
        let head = git_dir.join(HEAD_FILE);
        if !head.is_file() {
            return false;
        }

        // 检查objects目录是否存在且为目录
        let objects = git_dir.join(OBJECTS_DIR);
        if !objects.is_dir() {
            return false;
        }

        // 检查refs目录是否存在且为目录
        let refs = git_dir.join(REFS_DIR);
        if !refs.is_dir() {
            return false;
        }

        true
    }
    pub fn init(dir: &Path) -> Result<Repository, &str> {
        let s = dir.to_str().unwrap();
        println!("{s}");
        if !dir.exists() {
            return Err("Specified init dir don't exists");
        }
        let git_dir = dir.join(GIT_DIR);
        if git_dir.exists() {
            return Err("git directory already exists");
        }
         // Create .git directory
        fs::create_dir(&git_dir).map_err(|_| "Failed to create git directory")?;

        // Create objects directory
        let objects_dir = git_dir.join(OBJECTS_DIR);
        fs::create_dir(&objects_dir).map_err(|_| "Failed to create objects directory")?;

        // Create refs directory
        let refs_dir = git_dir.join(REFS_DIR);
        fs::create_dir(&refs_dir).map_err(|_| "Failed to create refs directory")?;

        // Create HEAD file and write initial content
        let head_file = git_dir.join(HEAD_FILE);
        
        let _ = fs::File::create(head_file).map_err(|_|"Failed to create head file");
        let work_dir = env::current_dir().map_err(|_| "Failed to get current working dir")?;
        let obj_db = match ObjectDB::new(&objects_dir) {
            Ok(obj_db) => obj_db,
            Err(_) => {
                return Err("Failed to create object db");
            }
        };
        Ok(Repository { dir: dir.to_path_buf(), git_dir: git_dir, work_dir: work_dir, obj_db: obj_db })
    }
    pub fn open(path: &str) -> Result<Repository, Box<dyn Error>> {
        todo!("open")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    #[test]
    fn test_git_init() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let repo = Repository::init(path).unwrap();
        assert_eq!(repo.dir, path);
        assert_eq!(repo.git_dir, path.join(GIT_DIR));
        assert!(Repository::is_vaild_git_dir(&repo.git_dir));
    }
    #[test]
    fn is_vaild_git_dir_works() {
        // Since this project itself is managed by git
        assert!(Repository::is_vaild_git_dir(Path::new(".git")));
        assert!(!Repository::is_vaild_git_dir(Path::new(".gi")));
        assert!(!Repository::is_vaild_git_dir(Path::new("./src")));
    }
}
