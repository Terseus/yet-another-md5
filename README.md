# yet-another-md5

An implementation of the [MD5](https://en.wikipedia.org/wiki/MD5) hash algorithm capable to hash data readed from a [std::io::Read] implementation.


## Why?

The main motivation of this project is as an exercise to learn some Rust.

The second one is that the MD5 implementations I found for Rust hashes binary strings, which forced you to have the whole data to hash in memory; this looked silly to me given that the MD5 algorithm hash the data in chunks.


## Usage

```rust
use std::fs::File;
use std::io::prelude::*;
use ya_md5::Md5Hasher;
use ya_md5::Hash;
use ya_md5::Md5Error;

fn main() -> Result<(), Md5Error> {
    let mut file = File::open("foo.txt")?;
    Md5Hasher::hash(&mut file)?
    let result = format!("{}", hash);
    assert_eq!(result, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    Ok(())
}
```

See [the docs](https://docs.rs/yet-another-md5).
