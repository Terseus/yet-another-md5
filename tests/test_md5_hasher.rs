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

// Values taken from RFC section "A.5 Test suite"
// https://www.ietf.org/rfc/rfc1321.txt
#[rstest]
#[case("", "d41d8cd98f00b204e9800998ecf8427e")]
#[case("a", "0cc175b9c0f1b6a831c399e269772661")]
#[case("abc", "900150983cd24fb0d6963f7d28e17f72")]
#[case("message digest", "f96b697d7cb7938d525a2f31aaf161d0")]
#[case("abcdefghijklmnopqrstuvwxyz", "c3fcd3d76192e4007dfb496cca67e13b")]
#[case(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
    "d174ab98d277d9f5a5611c2c9f419d9f"
)]
#[case(
    "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
    "57edf4a22be3c955ac49da2e2107b67a"
)]
#[case(
    "1234567890123456789012345678901234567890123456789012345",
    "c9ccf168914a1bcfc3229f1948e67da0"
)]
fn test_hash_str(#[case] data: &str, #[case] expected: &str) {
    let digest = Md5Hasher::hash_str(data);
    let result = format!("{}", digest);
    assert_eq!(result, expected);
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
