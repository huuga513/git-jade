use std::str::FromStr;

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
}
impl AsRef<EncodedSha> for EncodedSha {
    fn as_ref(&self) -> &EncodedSha {
        &self
    }
}
impl FromStr for EncodedSha {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 40 {
            return Err(())
        }
        Ok(EncodedSha(s.to_string()))
    }
}

impl ToString for EncodedSha {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}