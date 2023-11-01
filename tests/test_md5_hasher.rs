use anyhow::Result;
use libmd5::Md5Hasher;
use log::LevelFilter;
use rstest::rstest;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use tempfile::tempfile;

#[allow(unused)]
fn setup_logger() {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    );
}

#[rstest]
#[case(
    [
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    "d41d8cd98f00b204e9800998ecf8427e"
)]
#[case(
    [
        0x61, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    "0cc175b9c0f1b6a831c399e269772661"
)]
fn test_compute_single_chunk(#[case] chunk: [u8; 64], #[case] expected: &str) {
    let mut instance = Md5Hasher::new();
    instance.add_chunk(chunk);
    let digest = instance.compute();
    let result = format!("{}", digest);
    assert_eq!(result, expected);
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
    let digest = Md5Hasher::hash_str(&data);
    let result = format!("{}", digest);
    assert_eq!(result, expected);
}

#[rstest]
fn test_hash_input() -> Result<()> {
    let mut file = tempfile()?;
    write!(file, "abc")?;
    file.seek(SeekFrom::Start(0))?;
    let digest = Md5Hasher::hash(&mut file)?;
    let result = format!("{}", digest);
    assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
    Ok(())
}

#[rstest]
fn test_hash_slice() {
    let digest = Md5Hasher::hash_slice(&"abc".as_bytes());
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
