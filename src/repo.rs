use chrono::{FixedOffset, Utc};

use crate::object::{Author, Commit};
use walkdir::WalkDir;

use super::EncodedSha;
use super::index::{Index, TreeNode};
use super::object::{Blob, ObjectDB, ObjectType, Tree};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs, io, path};
const OBJECTS_DIR: &str = "objects";
const REFS_DIR: &str = "refs";
const HEADS_DIR: &str = "heads";
const MASTER_BRANCH_NAME: &str = "master";
const HEAD_FILE: &str = "HEAD";
const GIT_DIR: &str = ".git";
const INDEX_FILE: &str = "index";
const AUTHOR_NAME: &str = "Alice";
const AUTHOR_EMAIL: &str = "alice@wonderland.edu";

mod line_diff {
    pub fn line_diff(a: &str, b: &str) -> Vec<bool> {
        let a_lines: Vec<&str> = a.split('\n').collect();
        let b_lines: Vec<&str> = b.split('\n').collect();
        let max_len = std::cmp::max(a_lines.len(), b_lines.len());
        (0..max_len)
            .map(|i| a_lines.get(i) == b_lines.get(i))
            .collect()
    }
    pub fn group_ranges(bools: &[bool]) -> Vec<(usize, usize, bool)> {
        let mut result = Vec::new();
        if bools.is_empty() {
            return result;
        }

        let mut current_start = 0;
        let mut current_value = bools[0];

        for (i, &value) in bools.iter().enumerate().skip(1) {
            if value != current_value {
                result.push((current_start, i - 1, current_value));
                current_start = i;
                current_value = value;
            }
        }

        result.push((current_start, bools.len() - 1, current_value));
        result
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_line_diff_basic() {
            let a = "hello\nworld\n2023";
            let b = "hello\nworld\n2023";
            assert_eq!(line_diff(a, b), vec![true, true, true]);

            let a = "Rust\nis\nfast";
            let b = "Rust\nis\nslow";
            assert_eq!(line_diff(a, b), vec![true, true, false]);

            let a = "line1\nline2";
            let b = "line1";
            assert_eq!(line_diff(a, b), vec![true, false]);
        }

        #[test]
        fn test_line_diff_edge_cases() {
            assert_eq!(line_diff("", ""), vec![true] as Vec<bool>);
            assert_eq!(line_diff("text", ""), vec![false]);
            assert_eq!(line_diff("", "text"), vec![false]);

            let a = "first\n\nthird";
            let b = "first\nsecond\nthird";
            assert_eq!(line_diff(a, b), vec![true, false, true]);
        }

        #[test]
        fn test_group_ranges_normal() {
            let bools = vec![true, true, false, false, true];
            assert_eq!(
                group_ranges(&bools),
                vec![(0,1,true), (2,3,false), (4,4,true)]
            );

            let all_true = vec![true; 5];
            assert_eq!(
                group_ranges(&all_true),
                vec![(0,4,true)]
            );
        }

        #[test]
        fn test_group_ranges_special() {
            assert_eq!(group_ranges(&[true]), vec![(0,0,true)]);
            assert_eq!(group_ranges(&[false]), vec![(0,0,false)]);

            let empty: Vec<bool> = Vec::new();
            assert!(group_ranges(&empty).is_empty());

            let alternating = vec![true, false, true, false];
            assert_eq!(
                group_ranges(&alternating),
                vec![(0,0,true), (1,1,false), (2,2,true), (3,3,false)]
            );
        }

        #[test]
        fn test_integration() {
            let a = "A\nB\nC\nD";
            let b = "A\nX\nC\nY";
            let diff = line_diff(a, b);
            assert_eq!(diff, vec![true, false, true, false]);
            
            let groups = group_ranges(&diff);
            assert_eq!(
                groups,
                vec![(0,0,true), (1,1,false), (2,2,true), (3,3,false)]
            );
        }
    }
}
pub struct Repository {
    dir: PathBuf,      // Path to the repository directory.
    git_dir: PathBuf,  // Path to the git directory ({dir}/{GIT_DIR}).
    obj_db: ObjectDB,
}
/// Represents the difference status between two index entries
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexDiffType {
    /// Entry exists only in the left/index
    LeftOnly,
    /// Entry exists only in the right/index
    RightOnly,
    /// Entry exists in both but has differences
    Modified,
    /// Entry exists in both with identical content
    Unmodified,
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
    pub fn init(dir: &Path) -> Result<Repository, String> {
        if !dir.exists() {
            return Err("Specified init dir don't exists".to_owned());
        }
        let git_dir = dir.join(GIT_DIR);
        if git_dir.exists() {
            return Err("git directory already exists".to_owned());
        }
        // Create .git directory
        fs::create_dir(&git_dir).map_err(|_| "Failed to create git directory")?;

        // Create objects directory
        let objects_dir = git_dir.join(OBJECTS_DIR);
        fs::create_dir(&objects_dir).map_err(|_| "Failed to create objects directory")?;

        // Create refs directory
        let refs_dir = git_dir.join(REFS_DIR);
        fs::create_dir(&refs_dir).map_err(|_| "Failed to create refs directory")?;

        // Create refs/heads directory
        let heads_dir = refs_dir.join(HEADS_DIR);
        fs::create_dir(&heads_dir).map_err(|_| "Failed to create heads directory")?;

        // Create HEAD file and write initial content
        let head_path = git_dir.join(HEAD_FILE);
        // e.g: refs/heads/master
        let head = Head::Symbolic(Path::new(REFS_DIR).join(HEADS_DIR).join(MASTER_BRANCH_NAME));
        head.save(&head_path).map_err(|why| why.to_string())?;

        let obj_db = match ObjectDB::new(&objects_dir) {
            Ok(obj_db) => obj_db,
            Err(_) => {
                return Err("Failed to create object db".to_owned());
            }
        };
        let repo = Repository {
            dir: dir.to_path_buf(),
            git_dir: git_dir,
            obj_db: obj_db,
        };
        repo.branch("master");
        Ok(repo)
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
    /// Converts the index into tree objects and stores them in the object database,
    /// returning the SHA1 hash of the root tree.
    ///
    /// # Workflow
    /// 1. Loads the index file from `.git/index`
    /// 2. Recursively constructs tree objects from directory structure
    /// 3. Stores all tree objects in the object database
    ///
    /// # Returns
    /// - `Ok(EncodedSha)`: 40-character SHA1 hash of root tree
    /// - `Err(String)`: Error description if any operation fails
    fn write_tree(&self) -> Result<EncodedSha, String> {
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
                let subdir_tree_sha1 = self.write_tree_impl(child).unwrap();
                tree.add_entry(ObjectType::Tree, &subdir_tree_sha1, name);
            }
        }
        let sha = self.obj_db.store(&tree).map_err(|why| why.to_string())?;
        Ok(sha)
    }

    /// Creates an index by reading a tree object from the object database
    ///
    /// # Arguments
    /// * `tree_root` - SHA1 hash of the root tree object to read
    ///
    /// # Returns
    /// Result containing the populated Index or error string
    fn read_tree(&self, tree_root: &EncodedSha) -> Result<Index, String> {
        // Initialize empty index
        let mut index = Index::new();

        // Recursively collect all file paths and their corresponding SHA1 hashes
        let (path_vec, sha_vec) = self.collect_tree_files(&tree_root)?;
        debug_assert_eq!(path_vec.len(), sha_vec.len());

        // Populate index with collected entries
        let mut i = 0;
        for sha in sha_vec.into_iter() {
            index.update_entry(&path_vec[i], sha);
            i += 1;
        }
        Ok(index)
    }

    /// Computes differences between two indexes
    ///
    /// # Arguments
    /// * `lhs` - Left-hand side index to compare
    /// * `rhs` - Right-hand side index to compare
    ///
    /// # Returns
    /// HashMap mapping file paths to their difference status
    fn diff_index(&self, lhs: &Index, rhs: &Index) -> HashMap<String, IndexDiffType> {
        let mut diff: HashMap<String, IndexDiffType> = HashMap::new();

        // First pass: Mark all left-side entries as LeftOnly
        for (name, _) in lhs.collect_entries() {
            diff.insert(name, IndexDiffType::LeftOnly);
        }

        // Second pass: Update status for right-side entries
        for (name, _) in rhs.collect_entries() {
            diff.entry(name.clone())
                .and_modify(|status| {
                    // Compare SHA1 hashes to determine modification status
                    *status = if lhs.get_sha1(&name).unwrap() == rhs.get_sha1(&name).unwrap() {
                        IndexDiffType::Unmodified
                    } else {
                        IndexDiffType::Modified
                    }
                })
                .or_insert(IndexDiffType::RightOnly);
        }
        diff
    }

    /// Updates working directory to match the specified index
    ///
    /// # Arguments
    /// * `index` - Target index to check out
    fn checkout_index(&self, index: &Index) {
        let head = self.get_head().unwrap();

        // Build index from current commit's tree
        let current_commit_index = match head {
            Head::Symbolic(path_buf) => {
                self.read_branch_to_index(path_buf.file_name().unwrap().to_str().unwrap())
            },
            Head::Detached(encoded_sha) => todo!(),
        };

        // Calculate differences between current state and target index
        let diff = self.diff_index(&current_commit_index, index);

        // Prevent overwriting untracked files
        for (file, status) in diff.iter() {
            if let IndexDiffType::RightOnly = status {
                let path = self.dir.join(file);
                if path.exists() {
                    println!(
                        "There is an untracked file in the way; delete it, or add and commit it first."
                    );
                    std::process::exit(1);
                }
            }
        }

        // Apply changes to working directory
        for (file, status) in diff.iter() {
            let path = self.dir.join(file);
            match status {
                IndexDiffType::LeftOnly => {
                    // Remove deleted files
                    if let Err(why) = fs::remove_file(&path) {
                        println!("Cannot remove {}: {}", &path.to_str().unwrap(), why);
                    }
                    // Clean up empty parent directories
                    if let Some(dir) = path.parent() {
                        let _ = fs::remove_dir(dir);
                    }
                }
                IndexDiffType::RightOnly | IndexDiffType::Modified => {
                    // Write new/changed files
                    if let Some(sha) = index.get_sha1(file) {
                        let blob_data = self.obj_db.retrieve(sha).unwrap_or_else(|why| {
                            println!("{}", why.to_string());
                            std::process::exit(1);
                        });
                        let blob = Blob::deserialize(&blob_data).unwrap_or_else(|why| {
                            println!("{}", why.to_string());
                            std::process::exit(1);
                        });
                        // Ensure parent directories exist
                        if let Some(dir) = path.parent() {
                            if !dir.is_dir() {
                                if let Err(why) = fs::create_dir_all(dir) {
                                    println!("{}", why.to_string());
                                    std::process::exit(1);
                                }
                            }
                        }
                        // Write file contents
                        let mut file = File::create(path).unwrap_or_else(|why| {
                            println!("{}", why.to_string());
                            std::process::exit(1);
                        });
                        file.write_all(&blob.data).unwrap_or_else(|why| {
                            println!("{}", why.to_string());
                            std::process::exit(1);
                        })
                    }
                }
                IndexDiffType::Unmodified => (),
            }
        }
    }
    pub fn status(&self) {
        let head = self.get_head().unwrap_or_else(|| {
            println!("Failed to fetch head");
            std::process::exit(1);
        });
        let commit_sha = match head {
            Head::Symbolic(path_buf) => {
                let branch_name = path_buf
                    .file_name()
                    .unwrap_or_else(|| {
                        println!("Failed to get branch name");
                        std::process::exit(1);
                    })
                    .to_str()
                    .unwrap_or_else(|| {
                        println!("Failed to ture to str");
                        std::process::exit(1);
                    });
                let branch =
                    Branch::load(&self.git_dir.join(REFS_DIR).join(HEADS_DIR), branch_name)
                        .unwrap_or_else(|| {
                            println!("Failed to load branch");
                            std::process::exit(1);
                        });
                branch.commit_sha
            }
            Head::Detached(commit_sha) => {
                println!("HEAD detached at {commit_sha}");
                Some(commit_sha)
            }
        };
        if commit_sha.is_none() {
            println!("No commits yet");
            std::process::exit(0);
        }
        let commit_sha = commit_sha.unwrap();
        let current_commit_data = self.obj_db.retrieve(&commit_sha).unwrap_or_else(|why| {
            println!("commit {commit_sha} doesn't exist: {why}");
            std::process::exit(1);
        });
        let current_commit = Commit::deserialize(&current_commit_data).unwrap_or_else(|why| {
            println!("{why}");
            std::process::exit(1);
        });
        let index = Index::load(&self.git_dir.join(INDEX_FILE)).unwrap_or_else(|why| {
            println!("cannot find index: {why}");
            std::process::exit(1);
        });
        // Build index from current commit's tree
        let current_commit_index = self
            .read_tree(&current_commit.get_tree_sha())
            .unwrap_or_else(|why| {
                println!("{}", why.to_string());
                std::process::exit(1);
            });

        // Calculate differences between current state and target index
        let diff = self.diff_index(&current_commit_index, &index);
        for (name, status) in diff {
            match status {
                IndexDiffType::LeftOnly => {
                    println!("Deleted: {name}");
                }
                IndexDiffType::RightOnly => {
                    println!("New: {name}");
                }
                IndexDiffType::Modified => {
                    println!("Modified: {name}");
                }
                IndexDiffType::Unmodified => (),
            }
        }
    }

    fn load_commit(&self, encoded_sha: &EncodedSha) -> Commit {
        let data = self.obj_db.retrieve(encoded_sha).unwrap();
        let commit = Commit::deserialize(&data).unwrap();
        commit
    }

    fn get_index_path(&self) -> PathBuf {
        self.git_dir.join(INDEX_FILE)
    }

    pub fn merge(&self, branch_name: &str) {
        let current_commit_sha = self.get_current_commit().unwrap();
        let mut index = Index::load(&self.get_index_path()).unwrap();
        let current_commit_data = self.obj_db.retrieve(&current_commit_sha).unwrap();
        let current_commit = Commit::deserialize(&current_commit_data).unwrap();
        let current_commit_index = self.read_tree(&current_commit.get_tree_sha()).unwrap();
        let diff = self.diff_index(&current_commit_index, &index);
        for (_, status) in diff {
            if let IndexDiffType::Unmodified = status {
            } else {
                println!("You have uncommitted changes.");
                std::process::exit(1);
            }
        }
        let branch = match Branch::load(&self.git_dir.join(REFS_DIR).join(HEADS_DIR), branch_name) {
            Some(branch) => branch,
            None => {
                println!("A branch with that name does not exist.");
                std::process::exit(1);
            }
        };
        if branch.commit_sha.is_none() {
            println!("There is no commit in branch {}", &branch.name);
            std::process::exit(1);
        }
        let branch_commit_sha = branch.commit_sha.unwrap();
        if branch_commit_sha == current_commit_sha {
            println!("Cannot merge a branch with itself.");
            std::process::exit(1);
        }
        let lca = match self.find_lca(&current_commit_sha, &branch_commit_sha) {
            Some(encoded_sha) => encoded_sha,
            None => {
                println!(
                    "Cannot find lca of {} and {}",
                    &current_commit_sha, &branch_commit_sha
                );
                std::process::exit(1);
            }
        };
        if lca.eq(&current_commit_sha) {
            self.fast_forward(branch_name);
            return;
        }
        if lca.eq(&branch_commit_sha) {
            return;
        }

        let branch_commit = self.load_commit(&branch_commit_sha);
        let lca_commit = self.load_commit(&lca);

        let branch_index = self.read_tree(&branch_commit.get_tree_sha()).unwrap();
        let lca_index = self.read_tree(&lca_commit.get_tree_sha()).unwrap();

        let diff_lca_cur = self.diff_index(&lca_index, &current_commit_index);
        let diff_lca_branch = self.diff_index(&lca_index, &branch_index);
        let mut has_conflict = false;

        // Collect all unique files from both diffs
        let all_files: HashSet<_> = diff_lca_cur
            .keys()
            .chain(diff_lca_branch.keys())
            .cloned()
            .collect();

        // Merge logic for each file
        for file_path in all_files {
            let cur_status = diff_lca_cur.get(&file_path);
            let branch_status = diff_lca_branch.get(&file_path);

            if cur_status.is_some() && branch_status.is_some() {
                let cur_status = cur_status.unwrap();
                let branch_status = branch_status.unwrap();
                match (cur_status, branch_status) {
                    // Both modified differently - Conflict
                    // 8 Any files modified in different ways in the current and given branches are in conflict.
                    // 8.1 the contents of both are changed and different from other.
                    // 8.3 the file was absent at the split point and has different
                    // contents in the given and current branches.
                    (IndexDiffType::Modified, IndexDiffType::Modified)
                    | (IndexDiffType::RightOnly, IndexDiffType::RightOnly) => {
                        let cur_sha = current_commit_index.get_sha1(&file_path).unwrap();
                        let branch_sha = branch_index.get_sha1(&file_path).unwrap();
                        // 3. Any files that have been modified in both the current and given branch in the same way
                        // are left unchanged by the merge.
                        // 3.1. Both files now have the same content
                        if cur_sha != branch_sha {
                            self.handle_conflict(
                                Path::new(&file_path),
                                cur_sha,
                                branch_sha,
                                &mut index,
                            );
                            has_conflict = true;
                        }
                    }

                    // Current deleted, branch modified (or vice versa) - Conflict
                    // 8.2 the contents of one are changed and the other file is deleted
                    (IndexDiffType::LeftOnly, IndexDiffType::Modified)
                    | (IndexDiffType::Modified, IndexDiffType::LeftOnly) => {
                        let (blob_sha, is_cur_content) = if let IndexDiffType::LeftOnly = cur_status
                        {
                            (branch_index.get_sha1(&file_path).unwrap(), false)
                        } else {
                            (current_commit_index.get_sha1(&file_path).unwrap(), true)
                        };
                        self.handle_deletion_conflict(
                            Path::new(&file_path),
                            blob_sha,
                            is_cur_content,
                            &mut index,
                        );
                        has_conflict = true;
                    }

                    // 1. Any files that have been modified in the given branch since the split point,
                    // but not modified in the current branch since the split point should be changed to their versions in the given branch
                    (IndexDiffType::Unmodified, IndexDiffType::Modified) => {
                        let sha = branch_index.get_sha1(&file_path).unwrap();
                        index.update_entry(&file_path, sha.clone());
                    }

                    // 2. Any files that have been modified in the current branch but not in the given branch
                    // since the split point should stay as they are.
                    (IndexDiffType::Modified, IndexDiffType::Unmodified) => (),

                    // 3.2 Both files were both removed are left unchanged by the merge.
                    (IndexDiffType::LeftOnly, IndexDiffType::LeftOnly) => (),

                    // 6. Any files present at the split point, unmodified in the current branch,
                    // and absent in the given branch should be removed (and untracked).
                    (IndexDiffType::Unmodified, IndexDiffType::LeftOnly) => {
                        index.remove_entry(&file_path);
                    }

                    // 7. Any files present at the split point, unmodified in the given branch,
                    // and absent in the current branch should remain absent.
                    (IndexDiffType::LeftOnly, IndexDiffType::Unmodified) => (),

                    // Other cases
                    _ => (),
                }
            }
            if branch_status.is_none() {
                let cur_status = cur_status.unwrap();
                match cur_status {
                    // 4. Any files that were not present at the split point and are present only in the current branch
                    // should remain as they are.
                    IndexDiffType::RightOnly => (),
                    _ => unreachable!(),
                }
            }
            if cur_status.is_none() {
                let branch_status = branch_status.unwrap();
                // 5. Any files that were not present at the split point
                // and are present only in the given branch should be checked out and staged.
                match branch_status {
                    IndexDiffType::RightOnly => {
                        let sha = branch_index.get_sha1(&file_path).unwrap();
                        index.update_entry(file_path, sha.clone());
                    }
                    _ => unreachable!(),
                }
            }
        }

        // Write merged index and create commit
        if let Err(why) = index.save(&self.get_index_path()) {
            println!("{why}");
            std::process::exit(1);
        }
        // Update work dir
        self.checkout_index(&index);
        let tree_sha = self.write_tree().unwrap();
        let parents = vec![current_commit_sha, branch_commit_sha.clone()];
        let commit_sha = self
            .commit_tree(
                tree_sha,
                parents,
                &format!("Merge {}", branch_name),
                AUTHOR_NAME,
                AUTHOR_EMAIL,
            )
            .unwrap();
        self.update_head(&commit_sha);
        //if has_conflict {
            //println!("Encountered a merge conflict.");
        //}
    }

    fn load_blob(&self, encoded_sha: &EncodedSha) -> Blob {
        let blob_data = self.obj_db.retrieve(encoded_sha).unwrap();
        let blob = Blob::deserialize(&blob_data).unwrap();
        blob
    }

    fn handle_conflict_text(
        &self,
        path: &Path,
        cur_content: String,
        branch_content: String,
        index: &mut Index,
    ) {
        let a_lines: Vec<&str> = cur_content.split('\n').collect();
        let diff = line_diff::line_diff(&cur_content, &branch_content);
        let mut merged_content = String::new();
        let get_conflict_text = |cur_text:&str, branch_text:&str| {
            format!("<<<<<<< HEAD\n{}=======\n{}>>>>>>>", cur_text, branch_text)
        };
        merged_content += &get_conflict_text(&cur_content, &branch_content);

        /* Example:
        Merge conflict in test.txt: 1
        Merge conflict in test.txt: [3, 5]
        Merge conflict in test.txt: [7, 9]  */
        for group in line_diff::group_ranges(&diff) {
            if group.2 == true {
                continue;
            }
            let start = group.0 + 1;
            let end = group.1 + 1;
            if start > a_lines.len() {
                continue;
            }
            let end = std::cmp::min(end, a_lines.len());
            print!("Merge conflict in {}: ", path.file_name().unwrap().to_str().unwrap());
            if start == end {
                println!("{}", start);
            } else {
                println!("[{}, {}]", start, end);
            }
        }
        let blob = Blob {
            data: merged_content.into(),
        };
        let blob_sha = self.obj_db.store(&blob).unwrap();
        index.update_entry(path, blob_sha);
    }

    // Helper to handle content conflicts
    fn handle_conflict(
        &self,
        path: &Path,
        cur_blob_sha: &EncodedSha,
        branch_blob_sha: &EncodedSha,
        index: &mut Index,
    ) {
        let cur_content = String::from_utf8(self.load_blob(cur_blob_sha).data).unwrap();
        let branch_content = String::from_utf8(self.load_blob(branch_blob_sha).data).unwrap();
        self.handle_conflict_text(path, cur_content, branch_content, index);
    }

    // Helper to handle deletion conflicts
    fn handle_deletion_conflict(
        &self,
        path: &Path,
        blob_sha: &EncodedSha,
        is_cur_content: bool,
        index: &mut Index,
    ) {
        let content = String::from_utf8(self.load_blob(blob_sha).data).unwrap();
        let (cur_content, branch_content) = if is_cur_content {
            (content, String::new())
        } else {
            (String::new(), content)
        };
        self.handle_conflict_text(path, cur_content, branch_content, index);
    }
    fn fast_forward(&self, target_branch_name: &str) {
        let head = self.get_head().unwrap();
        let target_branch = self.load_branch(target_branch_name).unwrap();
        let branch_dir = self.get_branch_dir();
        self.checkout(target_branch_name);
        let head = match head {
            Head::Symbolic(p) => {
                let current_branch = Branch::load(
                    &self.get_branch_dir(),
                    p.file_name().unwrap().to_str().unwrap(),
                )
                .unwrap();
                let current_branch = Branch {
                    commit_sha: target_branch.commit_sha,
                    ..current_branch
                };
                if let Err(why) = current_branch.save(&branch_dir) {
                    println!("Failed to save current branch: {why}");
                    std::process::exit(1);
                }
                Head::Symbolic(p)
            }
            Head::Detached(_) => Head::Detached(target_branch.commit_sha.unwrap()),
        };
        head.save(&self.git_dir.join(HEAD_FILE)).unwrap();
    }
    fn find_lca(&self, lhs: &EncodedSha, rhs: &EncodedSha) -> Option<EncodedSha> {
        let get_first_parent = |commit: &Commit| match commit.get_parents().first() {
            Some(encoded_sha) => Some(encoded_sha.clone()),
            None => None,
        };
        // Mark ancestors of lhs
        let mut ancestors = HashSet::new();
        let mut current = lhs.0.clone();
        loop {
            // Load commit data
            let commit_data = self
                .obj_db
                .retrieve(EncodedSha::from_string(current.clone()))
                .unwrap();
            let commit = Commit::deserialize(&commit_data).unwrap();
            ancestors.insert(current);
            if commit.get_parents().is_empty() {
                break;
            }
            current = match get_first_parent(&commit) {
                Some(encoded_sha) => encoded_sha.0.clone(),
                None => break, // Handle missing parent (e.g., invalid commit)
            };
        }
        // Traverse rhs ancestors to find LCA
        let mut current_rhs = rhs.0.clone();
        loop {
            // Check if current rhs commit is in lhs' ancestors
            if ancestors.contains(&current_rhs) {
                return Some(EncodedSha::from_string(current_rhs));
            }

            // Move to parent commit
            let commit_data = self
                .obj_db
                .retrieve(EncodedSha::from_string(current_rhs.clone()))
                .unwrap();
            let commit = Commit::deserialize(&commit_data).unwrap();

            match get_first_parent(&commit) {
                Some(parent_sha) => current_rhs = parent_sha.0,
                None => {
                    break;
                } // Reached root commit
            };
        }
        None
    }
    fn load_branch(&self, branch_name: &str) -> Option<Branch> {
        // Load branch metadata
        let branch = Branch::load(&self.git_dir.join(REFS_DIR).join(HEADS_DIR), branch_name);
        branch
    }

    fn read_branch_to_index(&self, branch_name: &str) -> Index {
        let branch = match self.load_branch(branch_name) {
            Some(b) => b,
            None => {
                println!("No such branch exists.");
                std::process::exit(1);
            }
        };
        let commit_sha = branch.commit_sha;
        if commit_sha.is_some() {
            let commit_sha = commit_sha.unwrap();

            // Load commit data
            let commit_data = self.obj_db.retrieve(commit_sha).unwrap();
            let commit = Commit::deserialize(&commit_data).unwrap();

            // Build index from commit's tree
            let tree_sha = commit.get_tree_sha();
            let index = self.read_tree(&tree_sha).unwrap_or_else(|why| {
                println!("{why}");
                std::process::exit(1);
            });
            return index;
        } else {
            // empty branch
            // remove all files checked by current index
            let index = Index::new();
            return index;
        }

    }

    /// Checks out a branch by updating HEAD and working directory
    ///
    /// # Arguments
    /// * `branch_name` - Name of the branch to check out
    pub fn checkout(&self, branch_name: &str) {
        let branch = match self.load_branch(branch_name) {
            Some(b) => b,
            None => {
                println!("No such branch exists.");
                std::process::exit(1);
            }
        };
        if let Some(head) = self.get_head() {
            if let Head::Symbolic(current_branch_path) = head {
                if current_branch_path.file_name().unwrap().to_str().unwrap() == &branch.name {
                    //println!("No need to checkout current branch");
                    std::process::exit(0);
                }
            }
        }

        let head = Head::Symbolic(Path::new(REFS_DIR).join(HEADS_DIR).join(branch.name));

        let index = self.read_branch_to_index(branch_name);
        // Update working directory
        self.checkout_index(&index);

        // Save index state
        index
            .save(&self.git_dir.join(INDEX_FILE))
            .unwrap_or_else(|why| {
                println!("{why}");
                std::process::exit(1);
            });
        head.save(&self.git_dir.join(HEAD_FILE)).unwrap();
    }

    /// Recursively collects all file entries from a tree object
    ///
    /// # Arguments
    /// * `tree_sha` - SHA1 hash of the tree object to process
    ///
    /// # Returns
    /// Tuple containing:
    /// - Vector of relative file paths
    /// - Vector of corresponding SHA1 hashes
    fn collect_tree_files(
        &self,
        tree_sha: &EncodedSha,
    ) -> Result<(Vec<PathBuf>, Vec<EncodedSha>), String> {
        // Retrieve and deserialize tree object
        let tree_data = self
            .obj_db
            .retrieve(tree_sha)
            .map_err(|why| why.to_string())?;
        let tree = Tree::deserialize(&tree_data).map_err(|why| why.to_string())?;

        let mut path_vec: Vec<PathBuf> = Vec::new();
        let mut sha_vec: Vec<EncodedSha> = Vec::new();

        // Process each entry in the tree
        for (name, entry) in tree.get_entries() {
            match entry.object_type {
                ObjectType::Blob => {
                    // Add file entry directly
                    path_vec.push(PathBuf::from_str(name).map_err(|why| why.to_string())?);
                    sha_vec.push(entry.sha1.clone());
                }
                ObjectType::Tree => {
                    // Recursively process subtree
                    let (sub_tree_path_vec, sub_tree_sha_vec) =
                        self.collect_tree_files(&entry.sha1)?;
                    // Merge subtree results with current paths
                    for sha in sub_tree_sha_vec {
                        sha_vec.push(sha);
                    }
                    for path in sub_tree_path_vec {
                        path_vec.push(Path::new(name).join(path));
                    }
                }
                ObjectType::Commit => {
                    return Err(format!("Commit type should not appear in a tree"));
                }
            }
        }
        Ok((path_vec, sha_vec))
    }
    /// Creates a commit object from a tree SHA and parent commits,
    /// then stores it in the object database.
    ///
    /// # Arguments
    /// * `tree_sha` - SHA1 hash of the tree object representing the snapshot
    /// * `parents` - List of parent commit SHA1s (empty for initial commit)
    /// * `message` - Commit message
    /// * `author_name` - Config user.name for author
    /// * `author_email` - Config user.email for author
    ///
    /// # Returns
    /// SHA1 hash of the created commit object
    fn commit_tree(
        &self,
        tree_sha: EncodedSha,
        parents: Vec<EncodedSha>,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<EncodedSha, String> {
        // Generate timestamp with current time and local offset
        let now = Utc::now();
        let offset = FixedOffset::east_opt(8 * 3600).unwrap(); // Use actual local offset
        let timestamp = now.with_timezone(&offset);

        // Create author/committer (usually same unless amended)
        let author = Author::new(author_name, author_email, timestamp);
        let committer = author.clone();

        // Build commit object
        let commit = Commit::new(tree_sha, parents, author, committer, message);

        // Store in object database and return SHA1
        Ok(self.obj_db.store(&commit).map_err(|e| e.to_string())?)
    }

    /// Attempts to load and return the HEAD reference from the .git directory.
    /// Returns `Some(Head)` if successfully loaded, or `None` on error.
    fn get_head(&self) -> Option<Head> {
        let head_path = self.git_dir.join(HEAD_FILE);
        let head = Head::load(&head_path).ok();
        head
    }

    /// Resolves and returns the SHA1 hash of the current commit.
    /// - For symbolic references (branches): Follows the branch pointer
    /// - For detached HEAD states: Directly returns the commit SHA1
    /// Panics if HEAD cannot be resolved or branch data is corrupted.
    /// If there is no commit found (e.g: just after git init), None is returned.
    fn get_current_commit(&self) -> Option<EncodedSha> {
        let head = self.get_head().unwrap();
        match head {
            Head::Symbolic(path_buf) => {
                let branch_path = self.git_dir.join(path_buf);
                let branch_result = Branch::load(
                    &branch_path.parent().unwrap(),
                    branch_path.file_name().unwrap().to_str().unwrap(),
                );
                match branch_result {
                    Some(branch) => branch.commit_sha,
                    None => None,
                }
            }
            Head::Detached(encoded_sha) => Some(encoded_sha),
        }
    }

    fn get_branch_dir(&self) -> PathBuf {
        self.git_dir.join(REFS_DIR).join(HEADS_DIR)
    }

    /// Creates a new branch pointing to the current commit.
    /// - Checks for existing branch name conflicts
    /// - Exits process if branch already exists
    /// - Saves new branch reference in .git/refs/heads/
    pub fn branch<S: AsRef<str>>(&self, name: S) {
        let branch_dir = self.get_branch_dir();
        match Branch::load(&branch_dir, name.as_ref()) {
            Some(_) => {
                println!("A branch with that name already exists.");
                std::process::exit(0);
            }
            None => {}
        };
        let current_commit = self.get_current_commit();
        let branch = Branch {
            name: name.as_ref().to_string(),
            commit_sha: current_commit,
        };
        branch.save(&branch_dir).unwrap();
    }

    /// Deletes an existing branch.
    /// - Prevents deletion of currently checked-out branch
    /// - Exits process if attempting to delete active branch
    /// - Removes branch reference from .git/refs/heads/
    pub fn rm_branch<S: AsRef<str>>(&self, name: S) {
        let head = self.get_head().unwrap();
        match head {
            Head::Symbolic(path_buf) => {
                if path_buf.file_name().unwrap().to_str().unwrap() == name.as_ref() {
                    println!("Cannot delete the currently active branch.");
                    std::process::exit(0);
                }
            }
            Head::Detached(_) => (),
        }
        let branch_dir = self.git_dir.join(REFS_DIR).join(HEADS_DIR);
        Branch::remove(&branch_dir, name.as_ref()).unwrap()
    }

    /// Stages file changes to the index (staging area).
    /// Accepts a list of file paths and updates their entries in the index.
    pub fn add<S: AsRef<str>>(&self, files: &Vec<S>) {
        let add_single_file = |p: &Path| {
            self.update_index(p).unwrap_or_else(|why| {
                println!("{why}");
                std::process::exit(1);
            })
        };
        for file in files {
            let file_path = Path::new(file.as_ref());
            if file_path.is_dir() {
                for entry in WalkDir::new(file_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|f| f.file_type().is_file())
                    .filter(|f| self.is_file_path_vaild(f.path()))
                {
                    add_single_file(entry.path());
                }
            } else {
                add_single_file(file_path);
            }
        }
    }

    pub fn rm<S: AsRef<str>>(&self, files: &Vec<S>) {
        let add_single_file = |p: &Path| {
            self.update_index(p).unwrap_or_else(|why| {
                println!("{why}");
                std::process::exit(1);
            })
        };
        let rm_single_file = |p: &Path| {
            let index= Index::load(&self.get_index_path()).unwrap();
            if index.get_sha1(p).is_none() {
                println!("fatal: pathspec '{}' did not match any files", p.to_str().unwrap());
                std::process::exit(1);
            }
            if let Err(why) = fs::remove_file(p) {
                println!("fatal: {}", why);
                std::process::exit(1);
            }
            add_single_file(p);
        };
        for file in files {
            let file_path = Path::new(file.as_ref());
            if file_path.is_dir() {
                for entry in WalkDir::new(file_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|f| f.file_type().is_file())
                    .filter(|f| self.is_file_path_vaild(f.path()))
                {
                    rm_single_file(entry.path());
                }
            } else {
                rm_single_file(file_path);
            }
        }
    }

    /// Creates a new commit with staged changes.
    /// - Validates non-empty commit message
    /// - Records parent commit, tree state, and author information
    /// - Updates HEAD reference (branch pointer or detached commit)
    /// Exits process if no changes detected or message is empty.
    pub fn commit<S: AsRef<str>>(&self, message: S) {
        // Convert the message to a string reference
        let message = message.as_ref();

        // Validate commit message is not empty
        if message.len() == 0 {
            println!("Please enter a commit message.");
            std::process::exit(0);
        }

        // Generate tree object from current index
        let tree = self.write_tree().unwrap();

        // Hardcoded author information (would normally be configurable)
        let author_name = AUTHOR_NAME;
        let author_email = AUTHOR_EMAIL;

        // Get parent commit if exists
        let parent = self.get_current_commit();

        // Create commit object, handling parent commit logic
        let commit_sha = match parent {
            Some(parent_sha) => {
                // Retrieve parent commit data from object database
                let parent_commit_data = self.obj_db.retrieve(&parent_sha).unwrap();
                let parent_commit = Commit::deserialize(&parent_commit_data).unwrap();

                // Prevent empty commits by comparing tree hashes
                if tree == parent_commit.get_tree_sha() {
                    println!("No changes added to the commit.");
                    std::process::exit(0);
                } else {
                    // Create commit with parent reference
                    self.commit_tree(tree, vec![parent_sha], message, author_name, author_email)
                        .unwrap()
                }
            }
            // Initial commit (no parent)
            None => self
                .commit_tree(tree, vec![], message, author_name, author_email)
                .unwrap(),
        };
        self.update_head(&commit_sha);
        eprintln!("{}", &commit_sha.0);
    }
    fn update_head(&self, commit_sha: &EncodedSha) {
        // Update HEAD reference
        let head = self.get_head().unwrap();
        let new_head = match &head {
            // Handle branch reference (symbolic HEAD)
            Head::Symbolic(path) => {
                // Create branch object with new commit
                let branch = Branch {
                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                    commit_sha: Some(commit_sha.clone()),
                };

                // Save updated branch reference
                branch
                    .save(&self.git_dir.join(path.parent().unwrap()))
                    .unwrap();
                head
            }
            // Handle detached HEAD state
            Head::Detached(_) => Head::Detached(commit_sha.clone()),
        };
        // Persist HEAD state to file
        new_head.save(&self.git_dir.join(HEAD_FILE)).unwrap();
    }
}

#[derive(Debug)]
struct Branch {
    name: String,
    commit_sha: Option<EncodedSha>,
}

impl Branch {
    /// save branch to base_path/\<branch name\>
    /// e.g: refs/heads/master
    pub fn save(&self, base_path: &Path) -> io::Result<()> {
        let file_path = base_path.join(&self.name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(encoded_sha) = &self.commit_sha {
            fs::write(file_path, encoded_sha.to_string())?;
        } else {
            fs::write(file_path, "No commit")?;
        }
        Ok(())
    }

    /// load branch from base_path/name
    pub fn load(base_path: &Path, name: &str) -> Option<Branch> {
        let file_path = base_path.join(name);
        let content = fs::read_to_string(file_path).ok()?;
        let commit_str = content.trim();
        let commit = EncodedSha::from_str(commit_str).ok();
        Some(Self {
            name: name.to_string(),
            commit_sha: commit,
        })
    }
    /// Removes the branch file from the specified base directory.
    ///
    /// # Arguments
    /// * `base_path` - The directory containing branch files
    ///
    /// # Returns
    /// * `io::Result<()>` - Success if file is deleted, error if deletion fails
    pub fn remove(base_path: &Path, name: &str) -> io::Result<()> {
        let file_path = base_path.join(&name);
        fs::remove_file(file_path)
    }
}

enum Head {
    /// Symbolic reference, e.g., refs/heads/master
    Symbolic(PathBuf),
    /// Detached HEAD state, directly pointing to a commit
    Detached(EncodedSha),
}

impl Head {
    /// Saves the HEAD to the specified path
    pub fn save(&self, path: &Path) -> io::Result<()> {
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Generate content based on state
        let content = match self {
            Head::Symbolic(ref_path) => format!("ref: {}\n", ref_path.display()),
            Head::Detached(sha) => sha.0.clone(),
        };

        fs::write(path, content)
    }

    /// Loads HEAD from the specified path
    pub fn load(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let content = content.trim();

        // Parse symbolic reference
        if let Some(stripped) = content.strip_prefix("ref: ") {
            Ok(Head::Symbolic(PathBuf::from(stripped)))
        }
        // Parse detached HEAD state
        else {
            Ok(EncodedSha::from_str(content)
                .map(Head::Detached)
                .map_err(|_| io::ErrorKind::InvalidData)?)
        }
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

#[cfg(test)]
mod function_tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_repo() -> Repository {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo
    }

    #[test]
    fn create_initial_commit() {
        let repo = create_test_repo();
        let tree_sha = EncodedSha::from_str("b45ef6fec89518d314f546fd3b302bf7a11b0d18").unwrap();

        let result = repo.commit_tree(
            tree_sha,
            vec![],
            "Initial commit",
            "Alice",
            "alice@example.com",
        );

        assert!(result.is_ok());
        let commit_sha = result.unwrap();

        // Verify commit exists in object database
        assert!(repo.obj_db.retrieve(&commit_sha).is_ok());
    }

    #[test]
    fn create_merge_commit() {
        let repo = create_test_repo();
        let tree_sha = EncodedSha::from_str("d4b8e6d7f7c1b7e0e6a4b8e6d7f7c1b7e0e6a4b8").unwrap();
        let parents = vec![
            EncodedSha("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string()),
            EncodedSha("b45ef6fec89518d314f546fd3b302bf7a11b0d18".to_string()),
        ];

        let result = repo.commit_tree(
            tree_sha,
            parents.clone(),
            "Merge branch 'feature'",
            "Bob",
            "bob@company.com",
        );

        assert!(result.is_ok());
        let commit_sha = result.unwrap();

        // Verify parent relationships
        let commit_data = repo.obj_db.retrieve(&commit_sha).unwrap();
        let commit = Commit::deserialize(&commit_data).unwrap();
        assert_eq!(*commit.get_parents(), parents);
    }

    #[test]
    fn commit_structure_validation() {
        let repo = create_test_repo();
        let tree_sha = EncodedSha::from_str("b45ef6fec89518d314f546fd3b302bf7a11b0d18").unwrap();

        let sha = repo
            .commit_tree(
                tree_sha,
                vec![],
                "Test commit",
                "Charlie",
                "charlie@test.org",
            )
            .unwrap();

        // Raw commit content verification
        let raw_commit = repo.obj_db.retrieve(&sha).unwrap();
        let content = String::from_utf8(raw_commit).unwrap();

        assert!(content.starts_with("commit "));
        assert!(content.contains("tree b45ef6fec89518d314f546fd3b302bf7a11b0d18\n"));
        assert!(content.contains("author Charlie <charlie@test.org>"));
        assert!(content.contains("\n\nTest commit"));
    }
}
#[cfg(test)]
mod branch_tests {
    use super::*;
    use std::io;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_branch() {
        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Construct a test branch
        let branch = Branch {
            name: "test-branch".to_string(),
            commit_sha: Some(EncodedSha("a".repeat(40))),
        };

        // Test saving the branch
        branch.save(base_path).unwrap();

        // Verify the file exists and its content is correct
        let file_path = base_path.join("test-branch");
        assert!(file_path.exists());
        assert_eq!(
            fs::read_to_string(file_path).unwrap().trim(),
            "a".repeat(40)
        );

        // Test loading the branch
        let loaded_branch = Branch::load(base_path, "test-branch").unwrap();
        assert_eq!(loaded_branch.name, "test-branch");
        assert_eq!(
            loaded_branch.commit_sha.unwrap().to_string(),
            "a".repeat(40)
        );
    }

    #[test]
    fn test_save_creates_parent_directories() {
        // Test the automatic directory creation logic
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("sub/dir");

        let branch = Branch {
            name: "deep-branch".to_string(),
            commit_sha: Some(EncodedSha("b".repeat(40))),
        };

        // Save to a multi-level directory
        branch.save(&base_path).unwrap();

        // Verify the file path
        let file_path = base_path.join("deep-branch");
        assert!(file_path.exists());
    }

    #[test]
    fn test_load_nonexistent_file() {
        // Test loading a non-existent branch
        let temp_dir = TempDir::new().unwrap();
        let result = Branch::load(temp_dir.path(), "ghost-branch");

        assert!(result.is_none());
    }

    #[test]
    fn test_load_invalid_commit_hash() {
        // Test loading an invalid hash value
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid-branch");

        // Write invalid content
        fs::write(&file_path, "short-hash").unwrap();

        let result = Branch::load(temp_dir.path(), "invalid-branch");

        assert!(result.is_some());
        assert!(result.unwrap().commit_sha.is_none());
    }
    #[test]
    fn test_remove_existing_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path();
        let branch_name = "existing-branch";

        // Create a test branch file
        let file_path = base_path.join(branch_name);
        fs::write(&file_path, "a".repeat(40))?;

        // Remove the branch
        Branch::remove(base_path, branch_name)?;

        // Verify file deletion
        assert!(!file_path.exists());
        Ok(())
    }

    #[test]
    fn test_remove_nonexistent_branch() {
        let temp_dir = TempDir::new().unwrap();
        let result = Branch::remove(temp_dir.path(), "ghost-branch");

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_remove_with_invalid_name() {
        let temp_dir = TempDir::new().unwrap();

        // Test empty branch name
        let result = Branch::remove(temp_dir.path(), "");
        assert!(result.is_err());

        // Test name with invalid characters
        let result = Branch::remove(temp_dir.path(), "invalid/name@");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_in_subdirectory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().join("refs/heads");
        let branch_name = "feature-branch";

        // Create nested branch file
        fs::create_dir_all(&base_path)?;
        fs::write(base_path.join(branch_name), "b".repeat(40))?;

        // Remove the branch
        Branch::remove(&base_path, branch_name)?;

        // Verify parent directory still exists
        assert!(base_path.exists());
        Ok(())
    }
}
#[cfg(test)]
mod head_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_symbolic_head() {
        let temp_dir = TempDir::new().unwrap();
        let head_path = temp_dir.path().join("HEAD");

        // Test saving symbolic reference
        let head = Head::Symbolic(PathBuf::from("refs/heads/master"));
        head.save(&head_path).unwrap();

        // Verify file content
        assert_eq!(
            fs::read_to_string(&head_path).unwrap().trim(),
            "ref: refs/heads/master"
        );

        // Test loading
        let loaded = Head::load(&head_path).unwrap();
        assert!(matches!(loaded, Head::Symbolic(p) if p == PathBuf::from("refs/heads/master")));
    }

    #[test]
    fn test_detached_head() {
        let temp_dir = TempDir::new().unwrap();
        let head_path = temp_dir.path().join("HEAD");

        // Test saving detached HEAD state
        let sha = EncodedSha("a".repeat(40));
        let head = Head::Detached(sha);
        head.save(&head_path).unwrap();

        // Verify file content
        assert_eq!(
            fs::read_to_string(&head_path).unwrap().trim(),
            "a".repeat(40)
        );

        // Test loading
        let loaded = Head::load(&head_path).unwrap();
        assert!(matches!(loaded, Head::Detached(_)));
    }

    #[test]
    fn test_invalid_head() {
        let temp_dir = TempDir::new().unwrap();
        let head_path = temp_dir.path().join("HEAD");

        // Write invalid content
        fs::write(&head_path, "invalid_commit_hash").unwrap();

        let result = Head::load(&head_path);
        assert!(matches!(result, Err(e) if e.kind() == io::ErrorKind::InvalidData));
    }
}
