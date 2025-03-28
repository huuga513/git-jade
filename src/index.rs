use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::fmt;

/// Represents a snapshot of tracked files in repository
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