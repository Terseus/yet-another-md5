#![allow(clippy::items_after_test_module)]

use rstest::rstest;
use std::io;
use std::io::Seek;
use std::io::Write;
use tempfile::tempfile;
use ya_md5::Md5Error;
use ya_md5::Md5Hasher;

#[ctor::ctor]
fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[rstest]
fn test_hash_str() {
    let digest = Md5Hasher::hash_str("abc");
    let result = format!("{}", digest);
    assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
}

#[rstest]
fn test_hash_input() -> Result<(), Md5Error> {
    let mut file = tempfile()?;
    write!(file, "abc")?;
    file.seek(io::SeekFrom::Start(0))?;
    let digest = Md5Hasher::hash(&mut file)?;
    let result = format!("{}", digest);
    assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
    Ok(())
}

#[rstest]
fn test_hash_slice() {
    let digest = Md5Hasher::hash_slice("abc".as_bytes());
    let result = format!("{}", digest);
    assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
}

#[rstest]
fn test_hash_vec() {
    let data = Vec::from("abc".as_bytes());
    let digest = Md5Hasher::hash_vec(&data);
    let result = format!("{}", digest);
    assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
}

#[rstest]
fn test_update_finalize() {
    let mut hasher = Md5Hasher::default();
    hasher.update("abc".as_bytes());
    let digest = hasher.finalize();
    let result = format!("{}", digest);
    assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
}
