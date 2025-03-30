use std::{path::Display, str::FromStr};

pub use repo::Repository;
mod index;
mod object;
pub mod repo;
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
            return Err(());
        }
        Ok(EncodedSha(s.to_string()))
    }
}

impl std::fmt::Display for EncodedSha {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
