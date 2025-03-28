use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::fmt;

/// Represents a snapshot of tracked files in repository
 #[derive(Debug)]
pub struct Index {
    /// Using BTreeMap for ordered entries (path -> SHA1)
    entries: BTreeMap<String, String>,
}

impl Index {
    /// Create a new empty Index
    pub fn new() -> Self {
        Index { entries: BTreeMap::default() }
    }

    /// Add/update a file entry with normalized path
    /// Any incoming path should be a relative path to the repository dir
    pub fn update_entry<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        sha1: String,
    ) {
        let normalized = Self::normalize_path(file_path);
        self.entries.insert(normalized, sha1);
    }

    /// Remove an entry by path
    pub fn remove_entry<P: AsRef<Path>>(
        &mut self,
        file_path: P,
    ) -> Option<String> {
        let normalized = Self::normalize_path(file_path);
        self.entries.remove(&normalized)
    }

    /// Get SHA1 by file path (returns Option for missing entries)
    pub fn get_sha1<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Option<&String> {
        let normalized = Self::normalize_path(file_path);
        self.entries.get(&normalized)
    }

    /// Path normalization: handles OS-specific separators and redundant components
    /// Normalize paths to UNIX-style format and resolve relative components
    fn normalize_path<P: AsRef<Path>>(path: P) -> String {
        let mut normalized = String::new();

        // Convert the path to a unified forward slash format first
        let path_str = path.as_ref().to_string_lossy().replace('\\', "/");
        let normalized_path = Path::new(&path_str);

        for component in normalized_path.components() {
            match component {
                Component::Normal(s) => {
                    if !normalized.is_empty() {
                        normalized.push('/');
                    }
                    normalized.push_str(s.to_str().unwrap());
                }
                _ => {} // Ignore special components such as root directory
            }
        }

        normalized
    }

    pub fn load(index_path: &Path) -> Result<Index, String>{
        if !index_path.exists() {
            return Err(format!("index file {} not found", index_path.to_str().unwrap()));
        }
        let content = std::fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
        let mut entries = BTreeMap::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                return Err("Invalid index format".to_string());
            }
            entries.insert(parts[0].to_string(), parts[1].to_string());
        }
        Ok(Index { entries })
    }

    pub fn save(&self, index_path: &Path) -> Result<(), String> {
        let mut content = String::new();
        for (path, sha1) in &self.entries {
            content.push_str(&format!("{} {}\n", path, sha1));
        }
        std::fs::write(&index_path, content).map_err(|e| e.to_string())
    }
}

/// Display implementation for debugging
impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Index Entries:")?;
        for (path, sha1) in &self.entries {
            writeln!(f, "  {:<40} {}", path, sha1)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut index = Index::new();
        
        // Test adding entries
        index.update_entry("src/main.rs", "abcd1234".into());
        index.update_entry("docs/README.md", "efgh5678".into());
        
        // Test retrieval
        assert_eq!(index.get_sha1("src/main.rs"), Some(&"abcd1234".into()));
        assert_eq!(index.get_sha1("docs\\README.md"), Some(&"efgh5678".into())); // Test Windows path

        // Test update
        index.update_entry("src/main.rs", "newsha1".into());
        assert_eq!(index.get_sha1("src/main.rs"), Some(&"newsha1".into()));

        // Test removal
        assert!(index.remove_entry("docs/README.md").is_some());
        assert!(index.get_sha1("docs/README.md").is_none());
    }

    #[test]
    fn test_path_normalization() {
        let mut index = Index::new();
        
        // Test different path formats
        index.update_entry("dir\\subdir/file.txt", "sha".into());
        assert_eq!(
            index.get_sha1("dir/subdir/file.txt"), // UNIX path
            Some(&"sha".into())
        );

        index.update_entry("../parent.txt", "sha2".into());
        assert_eq!(
            index.get_sha1("parent.txt"), // Relative components resolved
            Some(&"sha2".into())
        );
    }
    use tempfile::NamedTempFile;
    use std::io::Write;

    /// Test loading non-existent index file
    #[test]
    fn test_load_missing_file() {
        let non_existent = Path::new("non_existent_index");
        let result = Index::load(non_existent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    /// Test loading valid index format
    #[test]
    fn test_load_valid_format() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "file1.txt abcde12345\nsubdir/file2.txt 67890fghij").unwrap();
        
        let index = Index::load(file.path()).unwrap();
        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries.get("file1.txt"), Some(&"abcde12345".to_string()));
        assert_eq!(index.entries.get("subdir/file2.txt"), Some(&"67890fghij".to_string()));
    }

    /// Test loading invalid index format
    #[test]
    fn test_load_invalid_format() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "bad_line_without_space").unwrap();
        
        let result = Index::load(file.path());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid index format");
    }

    /// Test saving normal index entries
    #[test]
    fn test_save_normal_entries() {
        let mut index = Index::new();
        index.entries.insert("a.txt".to_string(), "123".to_string());
        index.entries.insert("b/c.txt".to_string(), "456".to_string());

        let file = NamedTempFile::new().unwrap();
        index.save(file.path()).unwrap();

        let content = std::fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("a.txt 123\n"));
        assert!(content.contains("b/c.txt 456\n"));
    }

    /// Test saving empty index
    #[test]
    fn test_save_empty_index() {
        let index = Index::new();
        let file = NamedTempFile::new().unwrap();
        
        index.save(file.path()).unwrap();
        let content = std::fs::read_to_string(file.path()).unwrap();
        assert!(content.is_empty());
    }
}
#[cfg(test)]
mod path_normalization_tests {
    use super::*;

    #[test]
    fn handles_different_os_separators() {
        // Windows 风格路径
        assert_eq!(Index::normalize_path("dir\\subdir\\file.txt"), "dir/subdir/file.txt");
        // 混合风格路径
        assert_eq!(Index::normalize_path("mixed/dir\\file"), "mixed/dir/file");
    }

    #[test]
    fn collapses_redundant_components() {
        // 当前目录标记
        assert_eq!(Index::normalize_path("./src/main.rs"), "src/main.rs");
        // 多重分隔符
        assert_eq!(Index::normalize_path("dir//subdir///file.txt"), "dir/subdir/file.txt");
    }

    #[test]
    fn handles_edge_cases() {
        // 根目录文件
        assert_eq!(Index::normalize_path("/topfile.txt"), "topfile.txt");
        // 空路径（应当返回空字符串）
        assert_eq!(Index::normalize_path(""), "");
    }

    #[test]
    fn normalizes_relative_paths() {
        // 上层目录（根据实现可能保留或忽略）
        assert_eq!(Index::normalize_path("../parent.txt"), "parent.txt");
        // 复杂相对路径
        assert_eq!(Index::normalize_path("../../dir/../file"), "dir/file");
    }

    #[test]
    fn preserves_case_sensitivity() {
        // 区分大小写
        assert_eq!(Index::normalize_path("CaseSensitive.txt"), "CaseSensitive.txt");
        assert_ne!(Index::normalize_path("caseSENSITIVE.txt"), "casesensitive.txt");
    }

    #[test]
    fn normalizes_special_characters() {
        // 空格处理
        assert_eq!(Index::normalize_path("dir with space/file"), "dir with space/file");
        // Unicode 字符
        assert_eq!(Index::normalize_path("中文目录/文件.txt"), "中文目录/文件.txt");
    }
}