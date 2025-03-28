pub use repo::Repository;
pub mod repo;
mod object;
mod index;
#[derive(Debug, Clone, PartialEq)]
struct EncodedSha(String);
impl EncodedSha {
    fn from_string(string: String) -> EncodedSha {
        EncodedSha(string)
    }
    fn from_str(str: &str) -> EncodedSha {
        EncodedSha(str.to_string())
    }
}