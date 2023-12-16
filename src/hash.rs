use std::fmt::Display;

/// The hash computed by the [Md5Hasher](crate::Md5Hasher).
#[derive(Debug)]
pub struct Hash {
    value: [u8; 16],
}

impl From<[u8; 16]> for Hash {
    fn from(value: [u8; 16]) -> Hash {
        Hash { value }
    }
}

impl Display for Hash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for value in self.value.iter() {
            write!(formatter, "{:02x}", value)?
        }
        Ok(())
    }
}
