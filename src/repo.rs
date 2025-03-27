use std::error::Error;
use std::path::Path;
const OBJECTS_DIR: &str = "objects";
const REFS_DIR: &str = "refs";
const HEAD_FILE: &str = "HEAD";
const GIT_DIR: &str = ".git";

pub struct Repository {
    dir: String,
    work_dir: String,
}

impl Repository {
    pub fn is_vaild_git_dir(path: &str) -> bool {
        let git_dir = Path::new(path);

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
    pub fn init(path: &str) -> Result<Repository, Box<dyn Error>> {
        todo!("init")
    }
    pub fn open(path: &str) -> Result<Repository, Box<dyn Error>> {
        todo!("open")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_vaild_git_dir_works() {
        assert!(Repository::is_vaild_git_dir(".git"));
        assert!(!Repository::is_vaild_git_dir(".gi"));
        assert!(!Repository::is_vaild_git_dir("./src"));
    }
}
