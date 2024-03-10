use std::fmt::Display;
use thiserror::Error;

pub const CHUNK_SIZE_BYTES: usize = 64; // 512 / 8

pub type RawChunk = [u8; CHUNK_SIZE_BYTES];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk(pub RawChunk);

#[derive(Debug, Error)]
pub enum ChunkTryFromSliceError {
    #[error("Invalid slice length: {0}")]
    InvalidSize(usize),
    #[error("Error converting slice to chunk: {0}")]
    TryFromSliceError(String),
}

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

impl Default for Chunk {
    fn default() -> Self {
        Chunk([0; CHUNK_SIZE_BYTES])
    }
}

impl TryFrom<&[u8]> for Chunk {
    type Error = ChunkTryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != CHUNK_SIZE_BYTES {
            return Err(ChunkTryFromSliceError::InvalidSize(value.len()));
        }
        Ok(Chunk(<RawChunk>::try_from(value).map_err(|err| {
            ChunkTryFromSliceError::TryFromSliceError(err.to_string())
        })?))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_chunk_from_slice_exact(
    ) -> Result<(), <Chunk as std::convert::TryFrom<&'static [u8]>>::Error> {
        let result = Chunk::try_from([10; CHUNK_SIZE_BYTES].as_slice())?;
        assert_eq!(result, Chunk([10; CHUNK_SIZE_BYTES]));
        Ok(())
    }

    #[test]
    fn test_chunk_from_slice_smaller() {
        let result = Chunk::try_from([10; 2].as_slice()).map_err(|err| format!("{0}", err));
        assert_eq!(result, Err("Invalid slice length: 2".to_string()));
    }

    #[test]
    fn test_chunk_from_slice_bigger() {
        const SIZE: usize = CHUNK_SIZE_BYTES + 1;
        let result = Chunk::try_from([10; SIZE].as_slice()).map_err(|err| format!("{0}", err));
        assert_eq!(
            result,
            Err(format!("Invalid slice length: {0}", SIZE).to_string())
        );
    }
}
