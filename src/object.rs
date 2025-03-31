use super::EncodedSha;
use hex;
use memchr::memchr;
use sha1::{Digest, Sha1};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

// Object type enumeration
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ToString for ObjectType {
    fn to_string(&self) -> String {
        match self {
            ObjectType::Blob => "blob".to_string(),
            ObjectType::Commit => "commit".to_string(),
            ObjectType::Tree => "tree".to_string(),
        }
    }
}

pub trait Object {
    /// Serialize the object into byte sequence with format "{type} {size}\0{contents}"
    fn serialize(&self) -> Vec<u8>;

    /// Calculate SHA-1 hash (20 bytes) of the serialized object
    fn sha1(&self) -> [u8; 20] {
        let data = self.serialize();
        let mut hasher = Sha1::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        let mut result = [0u8; 20];
        result.copy_from_slice(&hash);
        result
    }

    /// Encode SHA-1 hash as hexadecimal string
    fn encoded_sha1(&self) -> String {
        hex::encode(self.sha1())
    }
}

/// Determine object type from byte stream
pub fn determine_object_type(data: &[u8]) -> Result<ObjectType, String> {
    // Validate header format
    let null_pos = memchr(0, data).ok_or("Data missing null character separator")?;
    let header = &data[..null_pos];

    // Parse type field
    let space_pos = memchr(b' ', header).ok_or("Header missing space separator")?;
    let type_part = &header[..space_pos];
    let type_str = std::str::from_utf8(type_part)
        .map_err(|e| format!("Type field UTF-8 decode failed: {}", e))?;

    // Match known types
    match type_str {
        "blob" => Ok(ObjectType::Blob),
        "tree" => Ok(ObjectType::Tree),
        "commit" => Ok(ObjectType::Commit),
        _ => Err(format!("Unknown object type: {}", type_str)),
    }
}
#[derive(Debug)]
pub struct Blob {
    pub data: Vec<u8>,
}

impl Object for Blob {
    /// Serialize the blob into the format: "blob {size}\0{contents}"
    fn serialize(&self) -> Vec<u8> {
        // Create header components
        let obj_type = "blob";
        let size = self.data.len().to_string();

        // Build the serialized byte sequence
        let mut serialized = Vec::new();
        serialized.extend(obj_type.as_bytes()); // Add type
        serialized.push(b' '); // Add space
        serialized.extend(size.as_bytes()); // Add size
        serialized.push(0); // Add null byte
        serialized.extend(&self.data); // Add contents

        serialized
    }
}

impl Blob {
    /// Creates a new Blob from a file path
    ///
    /// # Arguments
    /// * `path` - Path to a valid file
    ///
    /// # Returns
    /// - Ok(Blob) containing file data if successful
    /// - Err(String) with error message if:
    ///   - Path doesn't exist
    ///   - Path points to a directory
    ///   - File read fails
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Blob, String> {
        let path = path.as_ref();

        // Validate path existence
        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }

        // Validate path is a file
        if !path.is_file() {
            return Err(format!("Path is not a file: {}", path.display()));
        }

        // Read file content
        let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

        Ok(Blob { data })
    }
    /// Deserialize byte stream into Blob object
    /// Returns Blob on success, or String with error description on failure
    pub fn deserialize(data: &[u8]) -> Result<Blob, String> {
        // Find null character separator
        let null_pos =
            memchr(0, data).ok_or_else(|| "Missing null separator in blob data".to_string())?;

        // Split header and content
        let (header_bytes, contents_with_null) = data.split_at(null_pos);
        let contents = &contents_with_null[1..]; // Skip null character

        // Parse header information
        let header = std::str::from_utf8(header_bytes)
            .map_err(|e| format!("Invalid UTF-8 in header: {}", e))?;

        // Split type and size
        let (obj_type, size_str) = header
            .split_once(' ')
            .ok_or_else(|| format!("Malformed header: '{}'", header))?;

        // Validate object type
        if obj_type != "blob" {
            return Err(format!(
                "Invalid object type: expected 'blob', found '{}'",
                obj_type
            ));
        }

        // Parse content length
        let size = size_str
            .parse::<usize>()
            .map_err(|_| format!("Invalid size format: '{}'", size_str))?;

        // Validate content length
        if contents.len() != size {
            return Err(format!(
                "Size mismatch: header claims {} bytes, actual {} bytes",
                size,
                contents.len()
            ));
        }

        Ok(Blob {
            data: contents.to_vec(),
        })
    }
}

/// Tree entry structure containing metadata
#[derive(Debug)]
pub struct TreeEntry {
    pub object_type: ObjectType,
    pub sha1: EncodedSha,
    pub name: String,
}
impl Tree {
    pub fn get_entries(&self) -> impl Iterator<Item = (&String, &TreeEntry)> {
        self.entries.iter()
    }
    pub fn get_object_type<S: AsRef<str>>(&self, filename: S) -> Option<ObjectType> {
        let filename = filename.as_ref();
        let entry = match self.entries.get(filename) {
            None => {
                return None;
            }
            Some(entry) => entry,
        };
        Some(entry.object_type.clone())
    }

    pub fn get_encoded_sha<S: AsRef<str>>(&self, filename: S) -> Option<EncodedSha> {
        let filename = filename.as_ref();
        let entry = match self.entries.get(filename) {
            None => {
                return None;
            }
            Some(entry) => entry,
        };
        Some(entry.sha1.clone())
    }
    /// Deserialize a Tree from a byte vector following Git's tree format
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let input = std::str::from_utf8(data)?;

        // Split header and entries
        let (header, entries_str) = input
            .split_once('\0')
            .ok_or("Invalid format: missing null separator")?;

        // Validate header format "tree {size}"
        let (prefix, size_str) = header.split_once(' ').ok_or("Invalid header format")?;
        if prefix != "tree" {
            return Err("Invalid header: not a tree object".into());
        }

        // Parse expected content size
        let expected_size = size_str.parse::<usize>()?;
        if entries_str.len() != expected_size {
            return Err(format!(
                "Size mismatch: expected {}, actual {}",
                expected_size,
                entries_str.len()
            )
            .into());
        }

        let mut entries = BTreeMap::new();

        // Parse each entry
        for line in entries_str.split('\n').filter(|l| !l.is_empty()) {
            let mut parts = line.splitn(3, ' ');

            let object_type = match parts.next().ok_or("Missing object type")? {
                "blob" => ObjectType::Blob,
                "tree" => ObjectType::Tree,
                t => return Err(format!("Invalid object type: {}", t).into()),
            };

            let sha_str = parts.next().ok_or("Missing SHA hash")?;
            if sha_str.len() != 40 || !sha_str.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("Invalid SHA1 format: {}", sha_str).into());
            }
            let sha1 = EncodedSha(sha_str.to_string());

            let name = parts.next().ok_or("Missing filename")?.to_string();

            // Validate unique filenames
            if entries.contains_key(&name) {
                return Err(format!("Duplicate entry: {}", name).into());
            }

            entries.insert(
                name.clone(),
                TreeEntry {
                    object_type,
                    sha1,
                    name,
                },
            );
        }

        Ok(Tree { entries })
    }
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Add an entry to the tree with automatic sorting
    pub fn add_entry(&mut self, object_type: ObjectType, sha1: &EncodedSha, name: &String) {
        // Use BTreeMap to maintain sorted order by filename
        self.entries.insert(
            name.to_string(),
            TreeEntry {
                object_type: object_type.clone(),
                sha1: sha1.clone(),
                name: name.clone(),
            },
        );
    }
}

/// Main tree structure storing sorted entries
#[derive(Debug)]
pub struct Tree {
    entries: BTreeMap<String, TreeEntry>,
}
// Database structure
pub struct ObjectDB {
    path: PathBuf,
}

impl Object for Tree {
    /// Serialize tree following "tree {size}\0{entries}" format
    /// Entry format: "{type} {sha} {name}\n"
    fn serialize(&self) -> Vec<u8> {
        // Generate entry lines with sorted order
        let contents: Vec<u8> = self
            .entries
            .values()
            .flat_map(|entry| {
                format!(
                    "{} {} {}\n",
                    entry.object_type.to_string(),
                    entry.sha1.0,
                    entry.name
                )
                .into_bytes()
            })
            .collect();

        // Build header with size and null separator
        let header = format!("tree {}\0", contents.len());
        let mut data = header.into_bytes();
        data.extend(contents);
        data
    }
}

use chrono::{DateTime, FixedOffset, Utc};
use std::fmt::{Display, Formatter};

/// Structure for commit author/committer information
#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    timestamp: DateTime<FixedOffset>, // Timestamp with timezone
}

impl Author {
    pub fn new(name: &str, email: &str, timestamp: DateTime<FixedOffset>) -> Self {
        Self {
            name: name.to_string(),
            email: email.to_string(),
            timestamp,
        }
    }
}

impl Display for Author {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Example format: Alice <alice@example.com> 1627987956 +0800
        write!(
            f,
            "{} <{}> {}",
            self.name,
            self.email,
            self.timestamp.format("%s %z")
        )
    }
}

/// Git commit object structure
#[derive(Debug)]
pub struct Commit {
    tree_sha: EncodedSha,     // SHA1 of the top-level tree object
    parents: Vec<EncodedSha>, // List of parent commit SHA1s
    author: Author,           // Author information
    committer: Author,        // Committer information
    message: String,          // Commit message
}

impl Commit {
    pub fn new(
        tree_sha: EncodedSha,
        parents: Vec<EncodedSha>,
        author: Author,
        committer: Author,
        message: &str,
    ) -> Self {
        Self {
            tree_sha: tree_sha,
            parents,
            author,
            committer,
            message: message.to_string(),
        }
    }
    pub fn get_parents(&self) -> &Vec<EncodedSha> {
        &self.parents
    }
    pub fn get_tree_sha(&self) -> EncodedSha {
        self.tree_sha.clone()
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Build commit content
        writeln!(f, "tree {}", self.tree_sha)?;

        // Write parent commits (if any)
        for parent in &self.parents {
            writeln!(f, "parent {}", parent)?;
        }

        // Write author and committer information
        writeln!(f, "author {}", self.author)?;
        writeln!(f, "committer {}", self.committer)?;

        // Empty line to separate header and message
        writeln!(f)?;

        // Write commit message (preserving original line breaks)
        write!(f, "{}", self.message)
    }
}
impl Object for Commit {
    /// Serialize commit object following Git's object format:
    /// "commit {content_length}\0{header}{message}"
    fn serialize(&self) -> Vec<u8> {
        // Convert to string representation first
        let content = self.to_string();
        // Format header: "commit {content_length}\0"
        let header = format!("commit {}\0", content.len());

        // Combine header and content
        let mut bytes = Vec::with_capacity(header.len() + content.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(content.as_bytes());
        bytes
    }
}

impl Commit {
    /// Deserialize raw object data into a Commit instance
    ///
    /// # Format
    /// Expects data in "commit {size}\0{content}" format where content contains:
    /// - tree SHA
    /// - optional parent SHAs
    /// - author/committer lines
    /// - empty line
    /// - commit message
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        // Split header and content at null byte
        let null_pos = data
            .iter()
            .position(|&b| b == b'\0')
            .ok_or("Missing null byte separator")?;
        let (header, content) = data.split_at(null_pos);
        let content = &content[1..]; // Skip null byte

        // Parse header: "commit {size}"
        let header_str = std::str::from_utf8(header).map_err(|e| e.to_string())?;
        let (obj_type, obj_size) = parse_header(header_str)?;

        // Validate object type
        if obj_type != "commit" {
            return Err(format!("Expected commit object, got {}", obj_type));
        }

        // Verify content length matches header size
        if content.len() != obj_size {
            return Err(format!(
                "Size mismatch: header {} vs actual {}",
                obj_size,
                content.len()
            ));
        }

        // Parse commit content
        parse_commit_content(content)
    }
}

/// Helper to parse object header
fn parse_header(header: &str) -> Result<(&str, usize), String> {
    let mut parts = header.splitn(2, ' ');
    let obj_type = parts.next().ok_or("Missing object type")?;
    let obj_size = parts
        .next()
        .ok_or("Missing object size")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    Ok((obj_type, obj_size))
}

/// Helper to parse commit content
fn parse_commit_content(content: &[u8]) -> Result<Commit, String> {
    let content_str = std::str::from_utf8(content).map_err(|e| e.to_string())?;
    let mut lines = content_str.lines();

    let mut tree_sha = None;
    let mut parents = Vec::new();
    let mut author = None;
    let mut committer = None;
    let mut message = String::new();
    let mut in_message = false;

    // Parse header lines
    while let Some(line) = lines.next() {
        if line.is_empty() {
            in_message = true;
            continue;
        }

        if in_message {
            message.push_str(line);
            message.push('\n');
            continue;
        }

        if let Some(sha) = line.strip_prefix("tree ") {
            tree_sha = Some(sha.to_string());
        } else if let Some(parent_sha) = line.strip_prefix("parent ") {
            parents.push(EncodedSha(parent_sha.to_string()));
        } else if let Some(auth_info) = line.strip_prefix("author ") {
            author = Some(parse_author(auth_info)?);
        } else if let Some(committer_info) = line.strip_prefix("committer ") {
            committer = Some(parse_author(committer_info)?);
        } else {
            return Err(format!("Unexpected line: {}", line));
        }
    }

    // Validate required fields
    let tree_sha = tree_sha.ok_or("Missing tree SHA")?;
    let tree_sha = EncodedSha(tree_sha);
    let author = author.ok_or("Missing author")?;
    let committer = committer.ok_or("Missing committer")?;

    // Remove trailing newline from message
    let message = message.trim_end().to_string();

    Ok(Commit {
        tree_sha,
        parents,
        author,
        committer,
        message,
    })
}

/// Parse author/committer line format: "Name <email> timestamp timezone"
fn parse_author(s: &str) -> Result<Author, String> {
    let mut parts = s.rsplitn(3, ' ');
    let tz = parts.next().ok_or("Missing timezone")?;
    let timestamp = parts.next().ok_or("Missing timestamp")?;
    let rest = parts.next().ok_or("Missing name/email")?;

    // Parse timestamp with timezone
    let full_ts = format!("{} {}", timestamp, tz);
    let dt = DateTime::parse_from_str(&full_ts, "%s %z").map_err(|e| e.to_string())?;

    // Parse name and email
    let (name, email) = rest
        .split_once(" <")
        .and_then(|(name, rest)| rest.strip_suffix('>').map(|email| (name, email)))
        .ok_or("Malformed author/committer line")?;

    Ok(Author::new(name, email, dt))
}
impl ObjectDB {
    /// Create new object database
    pub fn new(path: &Path) -> Result<ObjectDB, &str> {
        if !path.is_dir() {
            return Err("Objects dir not exists!");
        }
        let path_buf = path.to_path_buf();
        Ok(ObjectDB { path: path_buf })
    }

    /// Store object in database
    pub fn store(&self, obj: &impl Object) -> std::io::Result<EncodedSha> {
        // Generate SHA1 hash
        let encoded_sha = obj.encoded_sha1();
        let (dir_part, file_part) = encoded_sha.split_at(2);

        // Build storage path
        let obj_dir = self.path.join(dir_part);
        let obj_path = obj_dir.join(file_part);

        // Avoid duplicate writes
        if !obj_path.exists() {
            // Create directory
            fs::create_dir_all(&obj_dir)?;

            // Write data
            let mut file = File::create(&obj_path)?;
            file.write_all(&obj.serialize())?;
        }

        Ok(EncodedSha(encoded_sha))
    }

    /// Retrieve object from database
    pub fn retrieve<E: AsRef<EncodedSha>>(&self, encoded_sha: E) -> std::io::Result<Vec<u8>> {
        // Validate SHA format
        let encoded_sha = &encoded_sha.as_ref().0;
        if encoded_sha.len() != 40 || !encoded_sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid SHA1 hash format",
            ));
        }

        // Parse path
        let (dir_part, file_part) = encoded_sha.split_at(2);
        let obj_path = self.path.join(dir_part).join(file_part);

        // Read file
        let mut file = File::open(obj_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        Ok(contents)
    }
}

#[cfg(test)]
mod blob_tests {
    use super::*;
    use tempfile::{NamedTempFile, tempdir};

    #[test]
    fn creates_blob_from_valid_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();

        let blob = Blob::new(file.path()).unwrap();
        assert_eq!(blob.data, b"test content");
    }

    #[test]
    fn rejects_missing_file() {
        let result = Blob::new("non_existent.file");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn rejects_directory_path() {
        let dir = tempdir().unwrap();
        let result = Blob::new(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a file"));
    }

    #[test]
    fn handles_unreadable_file() {
        #[cfg(unix)] // Test UNIX-style permissions
        {
            use std::os::unix::fs::PermissionsExt;

            let file = NamedTempFile::new().unwrap();
            let mut perms = file.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o000); // No permissions
            file.as_file().set_permissions(perms).unwrap();

            let result = Blob::new(file.path());
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Failed to read"));
        }
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;
    #[test]
    fn test_tree_serialization() {
        let mut tree = Tree::new();
        let entry1 = TreeEntry {
            object_type: ObjectType::Blob,
            sha1: EncodedSha {
                0: "a906cb2a4a904a152e80877d4088654daad0c859".to_string(),
            },
            name: "README".into(),
        };
        let entry2 = TreeEntry {
            object_type: ObjectType::Tree,
            sha1: EncodedSha {
                0: "99f1a6d12cb4b6f19c8655fca46c3ecf317074e0".to_string(),
            },
            name: "lib".into(),
        };
        // Add test entries
        tree.add_entry(entry1.object_type.clone(), &entry1.sha1, &entry1.name);

        tree.add_entry(entry2.object_type.clone(), &entry2.sha1, &entry2.name);

        // Verify serialization format
        let data = tree.serialize();
        let expected_content = format!(
            "{} {} {}\n{} {} {}\n",
            entry1.object_type.to_string(),
            entry1.sha1.0,
            entry1.name,
            entry2.object_type.to_string(),
            entry2.sha1.0,
            entry2.name
        );
        let expected_header = format!("tree {}\0", expected_content.len());

        assert!(data.starts_with(expected_header.as_bytes()));
        assert!(data.ends_with(expected_content.as_bytes()));
        let deserialized_tree = Tree::deserialize(&data).unwrap();
        assert_eq!(
            deserialized_tree.get_object_type(&entry1.name).unwrap(),
            entry1.object_type
        );
        assert_eq!(
            deserialized_tree.get_encoded_sha(&entry1.name).unwrap(),
            entry1.sha1
        );
        assert_eq!(
            deserialized_tree.get_object_type(&entry2.name).unwrap(),
            entry2.object_type
        );
        assert_eq!(
            deserialized_tree.get_encoded_sha(&entry2.name).unwrap(),
            entry2.sha1
        );
    }
    #[test]
    fn test_filename_with_spaces() {
        let data = b"tree 61\0blob 0000000000000000000000000000000000000000 file with space";

        let tree = Tree::deserialize(data).unwrap();
        let entry = tree.entries.get("file with space").unwrap();
        assert_eq!(entry.name, "file with space");
    }

    #[test]
    fn test_missing_null_separator() {
        let data = b"tree 100invalid_data";
        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_header_prefix() {
        let data = b"tre 0\0";
        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_size_mismatch() {
        let data = b"tree 100\0small_data";
        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_object_type() {
        let data = b"tree 46\0commit 0000000000000000000000000000000000000000 test";
        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_sha_format() {
        let data = b"tree 44\0blob invalid_sha test";
        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_filename() {
        let data = b"tree 94\0\
            blob 0000000000000000000000000000000000000000 dup\n\
            tree 0000000000000000000000000000000000000000 dup\n";

        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_fields() {
        let data = b"tree 30\0blob 0000000000000000000000000000000000000000";
        let result = Tree::deserialize(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_tree() {
        let data = b"tree 0\0";
        let tree = Tree::deserialize(data).unwrap();
        assert!(tree.entries.is_empty());
    }
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use tempfile::TempDir;

    struct TestObject(Vec<u8>);

    impl Object for TestObject {
        fn serialize(&self) -> Vec<u8> {
            self.0.clone()
        }

        // Other trait methods use default implementations
    }

    #[test]
    fn test_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let db = ObjectDB::new(temp_dir.path()).unwrap();

        // Test object
        let obj = TestObject(b"test data".to_vec());
        let sha = db.store(&obj).unwrap();
        let sha_ref = &sha;

        // Verify path structure
        let stored_path = db.path.join(&sha.0[..2]).join(&sha.0[2..]);
        assert!(stored_path.exists());

        // Read and verify
        let retrieved = db.retrieve(sha_ref).unwrap();
        assert_eq!(retrieved, obj.serialize());
    }

    #[test]
    fn test_idempotent_store() {
        let temp_dir = TempDir::new().unwrap();
        let db = ObjectDB::new(temp_dir.path()).unwrap();
        let obj = TestObject(vec![1, 2, 3]);

        // First store
        let sha1 = db.store(&obj).unwrap();
        // Second store
        let sha2 = db.store(&obj).unwrap();

        assert_eq!(sha1, sha2);
    }
    #[test]
    fn determine_type_works() {
        let blob_data = b"blob 12\0hello world";
        assert_eq!(determine_object_type(blob_data), Ok(ObjectType::Blob));

        let invalid_data = b"tag 5\0data";
        assert!(matches!(determine_object_type(invalid_data), Err(_)));
    }
    #[test]
    fn test_serialize_empty_blob() {
        let blob = Blob { data: vec![] };
        let serialized = blob.serialize();
        assert_eq!(serialized, b"blob 0\0");
    }

    #[test]
    fn test_serialize_ascii_content() {
        let blob = Blob {
            data: b"hello".to_vec(),
        };
        let serialized = blob.serialize();
        assert_eq!(serialized, b"blob 5\0hello");
    }

    #[test]
    fn test_serialize_binary_content() {
        let blob = Blob {
            data: vec![0x00, 0xFF, 0x42],
        };
        let serialized = blob.serialize();
        let expected = b"blob 3\0\x00\xFF\x42".to_vec();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_roundtrip() {
        let original_data = vec![1, 2, 3, 4, 5];
        let blob = Blob {
            data: original_data.clone(),
        };

        // Serialize and deserialize
        let serialized = blob.serialize();
        let deserialized = Blob::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.data, original_data);
    }

    #[test]
    fn test_large_size() {
        let data = vec![0u8; 10_000];
        let blob = Blob { data };
        let serialized = blob.serialize();

        // Verify header format
        let header_end = serialized.iter().position(|&b| b == 0).unwrap();
        let header = &serialized[..header_end];
        assert_eq!(header, b"blob 10000");
    }
}

#[cfg(test)]
mod commit_tests {
    use std::str::FromStr;

    use super::*;
    use chrono::TimeZone;
    fn create_sample_author() -> Author {
        let timestamp = FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2023, 7, 20, 10, 30, 0)
            .unwrap();

        Author::new("Alice", "alice@example.com", timestamp)
    }

    #[test]
    fn test_initial_commit() {
        let author = create_sample_author();
        let commit = Commit::new(
            EncodedSha::from_str("b45ef6fec89518d314f546fd3b302bf7a11b0d18").unwrap(),
            vec![],
            author.clone(),
            author,
            "Initial commit",
        );

        let expected = r#"tree b45ef6fec89518d314f546fd3b302bf7a11b0d18
author Alice <alice@example.com> 1689820200 +0800
committer Alice <alice@example.com> 1689820200 +0800

Initial commit"#;

        assert_eq!(commit.to_string(), expected);
    }

    #[test]
    fn test_commit_with_parents() {
        let author = create_sample_author();
        let commit = Commit::new(
            EncodedSha::from_str("d4b8e6d7f7c1b7e0e6a4b8e6d7f7c1b7e0e6a4b8").unwrap(),
            vec![
                EncodedSha("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string()),
                EncodedSha("b45ef6fec89518d314f546fd3b302bf7a11b0d18".to_string()),
            ],
            author.clone(),
            author,
            "Merge branch 'feature'\n\nAdd new functionality",
        );

        let expected = r#"tree d4b8e6d7f7c1b7e0e6a4b8e6d7f7c1b7e0e6a4b8
parent a94a8fe5ccb19ba61c4c0873d391e987982fbbd3
parent b45ef6fec89518d314f546fd3b302bf7a11b0d18
author Alice <alice@example.com> 1689820200 +0800
committer Alice <alice@example.com> 1689820200 +0800

Merge branch 'feature'

Add new functionality"#;

        assert_eq!(commit.to_string(), expected);
    }

    #[test]
    fn test_author_formatting() {
        let timestamp = FixedOffset::east_opt(-5 * 3600)
            .unwrap()
            .with_ymd_and_hms(2023, 7, 20, 10, 30, 0)
            .unwrap();

        let author = Author::new("Bob", "bob@company.com", timestamp);
        assert_eq!(author.to_string(), "Bob <bob@company.com> 1689867000 -0500");
    }
}
