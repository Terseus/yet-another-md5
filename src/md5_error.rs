use thiserror::Error;

/// The error of a [Md5Hasher](crate::Md5Hasher) operation.
#[derive(Error, Debug)]
pub enum Md5Error {
    /// Error while doing [read](std::io::Read::read) from an input.
    #[error("Error reading input: {0}")]
    ReadError(std::io::Error),
    /// Other I/O errors.
    /// While the [Md5Hasher](crate::Md5Hasher) don't use this error, it allows to create ergonomic
    /// functions for hashing a [std::io::Read] object.
    /// ```
    /// use std::fs::File;
    /// use std::io::prelude::*;
    /// use ya_md5::Md5Hasher;
    /// use ya_md5::Hash;
    /// use ya_md5::Md5Error;
    ///
    /// fn hash_file() -> Result<Hash, Md5Error> {
    ///     std::fs::write("foo.txt", b"hello world")?;
    ///     let hash = {
    ///         let mut file = File::open("foo.txt")?;
    ///         Md5Hasher::hash(&mut file)?
    ///     };
    ///     std::fs::remove_file("foo.txt")?;
    ///     Ok(hash)
    /// }
    ///
    /// ```
    #[error("Unexpected I/O error: {0}")]
    IOError(#[from] std::io::Error),
}
