use std::fmt::Display;

pub const CHUNK_SIZE_BYTES: usize = 64; // 512 / 8

pub type RawChunk = [u8; CHUNK_SIZE_BYTES];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk(pub RawChunk);

impl From<RawChunk> for Chunk {
    fn from(value: RawChunk) -> Self {
        Chunk(value)
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (index, byte) in self.0.iter().enumerate() {
            write!(f, "{:0>2x}", byte)?;
            if index + 1 < self.0.len() {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Chunk {
    pub fn empty() -> Self {
        Chunk([0; CHUNK_SIZE_BYTES])
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }
}
