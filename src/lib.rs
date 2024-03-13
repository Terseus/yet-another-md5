#![warn(missing_docs)]

//! An implementation of the [MD5] hash algorithm capable to hash data readed from a
//! [std::io::Read] implementation.
//!
//! ## Example
//! ```rust
//! use std::fs::File;
//! use std::io::prelude::*;
//! use ya_md5::Md5Hasher;
//! use ya_md5::Hash;
//! use ya_md5::Md5Error;
//!
//! fn example() -> Result<(), Md5Error> {
//!     std::fs::write("foo.txt", b"hello world")?;
//!     let hash = {
//!         let mut file = File::open("foo.txt")?;
//!         Md5Hasher::hash(&mut file)?
//!     };
//!     std::fs::remove_file("foo.txt")?;
//!     let result = format!("{}", hash);
//!     assert_eq!(result, "5eb63bbbe01eeed093cb22bb8f5acdc3");
//!     Ok(())
//! }
//! ```
//!
//! [MD5]: https://en.wikipedia.org/wiki/MD5

mod chunk;
mod chunk_processor;
mod conversions;
mod hash;
mod hash_compute_state;
mod md5_error;

use chunk::CHUNK_SIZE_BYTES;

pub use crate::hash::Hash;
pub use crate::md5_error::Md5Error;

use crate::chunk_processor::ChunkProcessor;

use std::io::Read;

/// A hasher thath computes the MD5 hash of a given list of chunks.
///
/// Each chunk is defined as a buffer of type `[u8; 64]`.
///
/// Provides conveniente functions to compute the MD5 hash of various sources without having to
/// create and manage an instance.
#[derive(Default)]
pub struct Md5Hasher {
    processor: ChunkProcessor,
}

impl Md5Hasher {
    /// Computes and returns the hash of the data that can be readed from the `input`.
    ///
    /// # Errors
    ///
    /// If there's any I/O error while reading the `input` an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use ya_md5::Md5Hasher;
    ///
    /// let hash = Md5Hasher::hash(&mut Cursor::new("hello world".as_bytes()))
    ///     .expect("Unexpected error reading from a cursor");
    /// let result = format!("{}", hash);
    /// assert_eq!(result, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    /// ```
    pub fn hash(input: &mut dyn Read) -> Result<Hash, Md5Error> {
        let mut hasher = Self::default();
        let mut buffer = [0; CHUNK_SIZE_BYTES];
        loop {
            let readed = input.read(&mut buffer).map_err(Md5Error::from)?;
            if readed == 0 {
                break;
            }
            hasher.update(&buffer[..readed]);
        }
        Ok(hasher.finalize())
    }

    /// Computes and returns the hash of the data in the slice.
    ///
    /// # Examples
    /// ```
    /// use ya_md5::Md5Hasher;
    ///
    /// let hash = Md5Hasher::hash_slice("hello world".as_bytes());
    /// let result = format!("{}", hash);
    /// assert_eq!(result, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    /// ```
    pub fn hash_slice(data: &[u8]) -> Hash {
        let mut hasher = Self::default();
        hasher.update(data);
        hasher.finalize()
    }

    /// Computes and returns the hash of the data in the `Vec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ya_md5::Md5Hasher;
    ///
    /// let hash = Md5Hasher::hash_vec(&Vec::from("hello world".as_bytes()));
    /// let result = format!("{}", hash);
    /// assert_eq!(result, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    /// ```
    pub fn hash_vec(data: &Vec<u8>) -> Hash {
        Self::hash_slice(data.as_slice())
    }

    /// Computes and returns the hash of the data in the string slice.
    ///
    /// # Examples
    /// ```
    /// use ya_md5::Md5Hasher;
    ///
    /// let hash = Md5Hasher::hash_str("hello world");
    /// let result = format!("{}", hash);
    /// assert_eq!(result, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    /// ```
    pub fn hash_str(data: &str) -> Hash {
        Self::hash_slice(data.as_bytes())
    }

    /// Process a single chunk and use it to compute the internal state.
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        self.processor.update(data);
    }

    /// Computes the hash of the internal state of the instance, consuming the instance in the
    /// process.
    pub fn finalize(self) -> Hash {
        self.processor.finalize()
    }
}
