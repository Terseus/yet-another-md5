mod chunk;
mod chunk_provider;
mod conversions;
mod hash;
mod hash_compute_state;

pub use crate::hash::Hash;

use crate::chunk::Chunk;
use crate::chunk::RawChunk;
use crate::chunk_provider::ChunkProvider;
use crate::hash_compute_state::HashComputeState;

use anyhow::Result;
use std::io::Cursor;
use std::io::Read;

pub struct Md5Hasher {
    state: HashComputeState,
}

impl Default for Md5Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5Hasher {
    pub fn new() -> Self {
        Md5Hasher {
            state: HashComputeState::new_initial(),
        }
    }

    pub fn hash(input: &mut dyn Read) -> Result<Hash> {
        let mut chunk_provider = ChunkProvider::new(input);
        let mut hasher = Md5Hasher::new();
        let mut buffer = Chunk::empty();
        while (chunk_provider.read(&mut buffer)?).is_some() {
            hasher.add_chunk_direct(buffer);
        }
        Ok(hasher.compute())
    }

    pub fn hash_vec(data: &Vec<u8>) -> Hash {
        Self::unsafe_hash(&mut Cursor::new(data))
    }

    pub fn hash_slice(data: &[u8]) -> Hash {
        Self::unsafe_hash(&mut Cursor::new(data))
    }

    pub fn hash_str(data: &str) -> Hash {
        Self::unsafe_hash(&mut Cursor::new(data.as_bytes()))
    }

    pub fn add_chunk(&mut self, chunk: RawChunk) {
        self.add_chunk_direct(Chunk::from(chunk))
    }

    pub fn compute(self) -> Hash {
        Hash::from(self.state.to_raw())
    }

    fn unsafe_hash(input: &mut dyn Read) -> Hash {
        match Self::hash(input) {
            Ok(value) => value,
            Err(_) => panic!("Error computing hash from static input"),
        }
    }

    fn add_chunk_direct(&mut self, chunk: Chunk) {
        self.state = self.state.process_chunk(&chunk);
    }
}
