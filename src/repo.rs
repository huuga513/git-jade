use super::index::{Index, TreeNode};
use super::object::{Blob, ObjectDB, ObjectType, Tree};
use core::error;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use std::path::{Path, PathBuf};
use std::{env, fs, path};
use super::EncodedSha;
const OBJECTS_DIR: &str = "objects";
const REFS_DIR: &str = "refs";
const HEAD_FILE: &str = "HEAD";
const GIT_DIR: &str = ".git";
const INDEX_FILE: &str = "index";

pub struct Repository {
    dir: PathBuf,      // Path to the repository directory.
    git_dir: PathBuf,  // Path to the git directory ({dir}/{GIT_DIR}).
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

        let _ = fs::File::create(head_file).map_err(|_| "Failed to create head file");
        let work_dir = env::current_dir().map_err(|_| "Failed to get current working dir")?;
        let obj_db = match ObjectDB::new(&objects_dir) {
            Ok(obj_db) => obj_db,
            Err(_) => {
                return Err("Failed to create object db");
            }
        };
        Ok(Repository {
            dir: dir.to_path_buf(),
            git_dir: git_dir,
            work_dir: work_dir,
            obj_db: obj_db,
        })
    }
    /// Open a repository based on the repository dir
    /// The git dir should be {dir}/{GIT_DIR}
    pub fn open(dir: &Path) -> Result<Repository, String> {
        let dir = path::absolute(dir).map_err(|_| "Failed to get dir abs path")?;
        let git_dir = dir.join(GIT_DIR);
        if !Repository::is_vaild_git_dir(&git_dir) {
            return Err(format!(
                "{} isn't a vaild git dir",
                git_dir.to_str().unwrap()
            ));
        }
        let work_dir = env::current_dir().map_err(|_| "Failed to get current working dir")?;
        let objects_dir = git_dir.join(OBJECTS_DIR);
        let obj_db = match ObjectDB::new(&objects_dir) {
            Ok(obj_db) => obj_db,
            Err(_) => {
                return Err("Failed to create object db".to_string());
            }
        };
        Ok(Repository {
            dir: dir.to_path_buf(),
            git_dir: git_dir,
            work_dir: work_dir,
            obj_db: obj_db,
        })
    }

    /// Validates if a file path meets repository requirements
    ///
    /// # Conditions
    /// 1. The path must be contained within the repository directory
    /// 2. The path must NOT be inside the .git directory
    ///
    /// # Returns
    /// - true: Path meets both conditions
    /// - false: Path violates either condition
    fn is_file_path_vaild(&self, file_path: &Path) -> bool {
        let abs_path = path::absolute(file_path).unwrap();
        // file path should in repository dir
        if !abs_path.starts_with(&self.dir) {
            return false;
        }
        // file path should not in git dir
        if abs_path.starts_with(&self.git_dir) {
            return false;
        }
        return true;
    }
    /// Converts an absolute path to repository-relative format
    ///
    /// # Parameters
    /// - file_path: Absolute path to convert
    ///
    /// # Returns
    /// - Ok(PathBuf): Relative path from repository root
    /// - Err(String): Original path isn't in repository directory
    ///
    /// # Example
    /// - Input: "/repo/foo/bar.txt"
    /// - Output: "foo/bar.txt" (when repo root is "/repo")
    fn turn_relative_path_to_repo_dir(&self, file_path: &Path) -> Result<PathBuf, String> {
        let abs_path = path::absolute(file_path).unwrap();
        match abs_path.strip_prefix(&self.dir) {
            Ok(relative_path) => Ok(relative_path.to_path_buf()),
            Err(why) => Err(why.to_string()),
        }
    }
    /// Updates the index with file changes
    ///
    /// # Workflow
    /// 1. Validate path security
    /// 2. Convert to repository-relative path
    /// 3. Handle index file existence
    /// 4. Update index entries based on file state:
    ///    - Existing file: Create/store blob + update entry
    ///    - Missing file: Remove existing entry
    fn update_index(&self, file_path: &Path) -> Result<(), String> {
        if !self.is_file_path_vaild(file_path) {
            return Err(format!(
                "File path {} invaild!",
                file_path.to_str().unwrap()
            ));
        }
        let entry_file_path = self
            .turn_relative_path_to_repo_dir(&file_path)?
            .to_str()
            .unwrap()
            .to_string();

        if file_path.exists() && !file_path.is_file() {
            return Err(format!("{} isn't a file", file_path.to_str().unwrap()));
        }

        let index_path = self.git_dir.join(INDEX_FILE);
        if !index_path.is_file() {
            let _ = fs::File::create_new(&index_path).map_err(|err| err.to_string());
        }
        let mut index = Index::load(&index_path)?;
        if file_path.exists() {
            let blob = Blob::new(&file_path)?;
            let sha1 = self.obj_db.store(&blob).map_err(|why| why.to_string())?;
            index.update_entry(&entry_file_path, sha1);
        } else {
            if index.get_sha1(&entry_file_path).is_some() {
                // delete the entry from index
                index.remove_entry(&entry_file_path);
            } else {
                return Err(format!(
                    "{} isn't a known file to git",
                    file_path.to_str().unwrap()
                ));
            }
        }
        index.save(&index_path)?;
        Ok(())
    }
    /// Convert index to tree objects and store them, returning root tree's SHA1
    pub fn write_tree(&self) -> Result<EncodedSha, String> {
        let index_path = self.git_dir.join(INDEX_FILE);
        let index = Index::load(&index_path)?;
        let root = index.get_root();
        self.write_tree_impl(root)
    }
    fn write_tree_impl(&self, node: &TreeNode) -> Result<EncodedSha, String> {
        let mut tree = Tree::new();
        for (name, child) in node.get_children() {
            if child.is_file() {
                tree.add_entry(ObjectType::Blob, &child.get_sha1().unwrap(), &name);
            } else {
                let subdir_tree_sha1 = self.write_tree_impl(child);
            }
        }
        todo!()
    }
    fn group_files_by_directory(
        file_paths: &[(&String, &String)],
    ) -> HashMap<String, Vec<(String, String)>> {
        let mut dir_map: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for &path in file_paths {
            let p = path::Path::new(path.0);
            let sha = path.1;

            // get parent dir (repository dir is repersented by '.')
            let parent = p.parent().unwrap_or_else(|| path::Path::new("."));

            let parent_str = parent.to_str().unwrap();
            // get filename
            if let Some(file_name) = p.file_name() {
                if let Some(name) = file_name.to_str() {
                    dir_map
                        .entry(parent_str.to_string())
                        .or_insert_with(Vec::new)
                        .push((name.to_string(), sha.to_string()));
                }
            }
        }

        dir_map
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
        assert!(!Repository::is_vaild_git_dir(Path::new("./target")));
    }

    use std::fs::{self, File};
    use std::io::Write;

    // Helper to create test files
    fn create_file(repo: &Repository, path: &str, content: &str) -> PathBuf {
        let full_path = repo.dir.join(path);
        let mut file = File::create(&full_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        full_path
    }

    #[test]
    fn test_update_index_add_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let file_path = create_file(&repo, "test.txt", "content");

        // First update (add)
        repo.update_index(&file_path).unwrap();
        let index = Index::load(&repo.git_dir.join("index")).unwrap();
        assert!(index.get_sha1("test.txt").is_some());
    }

    #[test]
    fn test_update_index_update_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let file_path = create_file(&repo, "update.txt", "v1");

        // First add
        repo.update_index(&file_path).unwrap();
        let index_path = repo.git_dir.join(INDEX_FILE);
        let index = Index::load(&index_path).unwrap();
        let original_sha = index.get_sha1("update.txt").unwrap().clone();

        // Update content
        create_file(&repo, "update.txt", "v2");
        repo.update_index(&file_path).unwrap();
        let index = Index::load(&index_path).unwrap();
        let new_sha = index.get_sha1("update.txt").unwrap();
        assert_ne!(&original_sha, new_sha);
    }

    #[test]
    fn test_update_index_remove_deleted_file() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let file_path = create_file(&repo, "to_delete.txt", "content");

        // Add to index
        repo.update_index(&file_path).unwrap();

        // Delete file and update
        fs::remove_file(&file_path).unwrap();
        repo.update_index(&file_path).unwrap();

        let index = Index::load(&repo.git_dir.join("index")).unwrap();
        assert!(index.get_sha1("to_delete.txt").is_none());
    }

    #[test]
    fn test_update_index_reject_unknown_file() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let bad_path = temp_dir.path().join("ghost.txt");

        let result = repo.update_index(&bad_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("isn't a known file"));
    }

    #[test]
    fn test_update_index_security_checks() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Test outside repo path
        let external_path = temp_dir.path().parent().unwrap().join("external.txt");
        let result = repo.update_index(&external_path);
        assert!(result.is_err());

        // Test .git directory path
        let git_path = repo.git_dir.join("config");
        let result = repo.update_index(&git_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_index_directory_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let dir_path = repo.dir.join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let result = repo.update_index(&dir_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("isn't a file"));
    }
}
