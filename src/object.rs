use std::{collections::BTreeMap, fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};
use super::EncodedSha;
use sha1::{Digest, Sha1};
use memchr::memchr;
use hex;

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
    data: Vec<u8>,
}

impl Object for Blob {
    /// Serialize the blob into the format: "blob {size}\0{contents}"
    fn serialize(&self) -> Vec<u8> {
        // Create header components
        let obj_type = "blob";
        let size = self.data.len().to_string();
        
        // Build the serialized byte sequence
        let mut serialized = Vec::new();
        serialized.extend(obj_type.as_bytes());  // Add type
        serialized.push(b' ');                   // Add space
        serialized.extend(size.as_bytes());      // Add size
        serialized.push(0);                      // Add null byte
        serialized.extend(&self.data);            // Add contents
        
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
        let data = fs::read(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        Ok(Blob { data })
    }
    /// Deserialize byte stream into Blob object
    /// Returns Blob on success, or String with error description on failure
    pub fn deserialize(data: &[u8]) -> Result<Blob, String> {
        // Find null character separator
        let null_pos = memchr(0, data).ok_or_else(|| 
            "Missing null separator in blob data".to_string()
        )?;

        // Split header and content
        let (header_bytes, contents_with_null) = data.split_at(null_pos);
        let contents = &contents_with_null[1..]; // Skip null character

        // Parse header information
        let header = std::str::from_utf8(header_bytes)
            .map_err(|e| format!("Invalid UTF-8 in header: {}", e))?;

        // Split type and size
        let (obj_type, size_str) = header.split_once(' ')
            .ok_or_else(|| format!("Malformed header: '{}'", header))?;

        // Validate object type
        if obj_type != "blob" {
            return Err(format!(
                "Invalid object type: expected 'blob', found '{}'", 
                obj_type
            ));
        }

        // Parse content length
        let size = size_str.parse::<usize>()
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
            data: contents.to_vec()
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
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Add an entry to the tree with automatic sorting
    pub fn add_entry(&mut self, object_type: ObjectType, sha1: &EncodedSha, name: &String) {
        // Use BTreeMap to maintain sorted order by filename
        self.entries.insert(name.to_string(), TreeEntry {
            object_type: object_type.clone(),
            sha1: sha1.clone(),
            name: name.clone(),
        });
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
    pub fn retrieve(&self, encoded_sha: &str) -> std::io::Result<Vec<u8>> {
        // Validate SHA format
        if encoded_sha.len() != 40 || !encoded_sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid SHA1 hash format"
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
            sha1: EncodedSha{0: "a906cb2a4a904a152e80877d4088654daad0c859".to_string()},
            name: "README".into(),
        };
        let entry2 = TreeEntry {
            object_type: ObjectType::Tree,
            sha1: EncodedSha{0:"99f1a6d12cb4b6f19c8655fca46c3ecf317074e0".to_string()},
            name: "lib".into(),
        };
        // Add test entries
        tree.add_entry(entry1.object_type.clone(), &entry1.sha1, &entry1.name);
        
        tree.add_entry(entry2.object_type.clone(), &entry2.sha1, &entry2.name);

        // Verify serialization format
        let data = tree.serialize();
        let expected_content = format!("{} {} {}\n{} {} {}\n", entry1.object_type.to_string(), entry1.sha1.0, entry1.name, entry2.object_type.to_string(), entry2.sha1.0, entry2.name);
        let expected_header = format!("tree {}\0", expected_content.len());
        println!("{}", std::str::from_utf8(&data).unwrap());
        
        assert!(data.starts_with(expected_header.as_bytes()));
        assert!(data.ends_with(expected_content.as_bytes()));
    }
}
#[cfg(test)]
mod tests {
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
        let sha = db.store(&obj).unwrap().0;

        // Verify path structure
        let stored_path = db.path.join(&sha[..2]).join(&sha[2..]);
        assert!(stored_path.exists());

        // Read and verify
        let retrieved = db.retrieve(&sha).unwrap();
        assert_eq!(retrieved, obj.serialize());
    }

    #[test]
    fn test_invalid_sha() {
        let temp_dir = TempDir::new().unwrap();
        let db = ObjectDB::new(temp_dir.path()).unwrap();

        // Short hash
        assert!(db.retrieve("abcd").is_err());
        // Invalid characters
        assert!(db.retrieve("z".repeat(40).as_str()).is_err());
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
        let blob = Blob { data: b"hello".to_vec() };
        let serialized = blob.serialize();
        assert_eq!(serialized, b"blob 5\0hello");
    }

    #[test]
    fn test_serialize_binary_content() {
        let blob = Blob { data: vec![0x00, 0xFF, 0x42] };
        let serialized = blob.serialize();
        let expected = b"blob 3\0\x00\xFF\x42".to_vec();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_roundtrip() {
        let original_data = vec![1, 2, 3, 4, 5];
        let blob = Blob { data: original_data.clone() };
        
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