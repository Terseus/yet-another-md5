use thiserror::Error;

#[derive(Error, Debug)]
pub enum Md5Error {
    #[error("Error reading input: {0}")]
    ReadError(std::io::Error),
}
