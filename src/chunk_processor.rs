use crate::chunk::{Chunk, CHUNK_SIZE_BYTES};
use crate::conversions::u64_to_u8;
use crate::hash::Hash;
use crate::hash_compute_state::HashComputeState;

const INITIAL_BIT_SIZE_BYTES: usize = 1;
const INITIAL_BIT: u8 = 0x80; // 1 in big endian.
const LENGTH_SIZE_BYTES: usize = 8; // 64 / 8
const ZERO_PADDING_MAX_SIZE_BYTES: usize =
    CHUNK_SIZE_BYTES - LENGTH_SIZE_BYTES - INITIAL_BIT_SIZE_BYTES;
const CHUNK_LENGTH: u64 = CHUNK_SIZE_BYTES as u64 * 8;

#[allow(dead_code)]
pub struct ChunkProcessor {
    buffer: Vec<u8>,
    state: HashComputeState,
    size: u64,
}

impl Default for ChunkProcessor {
    fn default() -> Self {
        ChunkProcessor {
            buffer: Vec::with_capacity(CHUNK_SIZE_BYTES),
            state: HashComputeState::default(),
            size: 0,
        }
    }
}

fn write_length(chunk: &mut Chunk, size: u64) {
    let mut length: [u8; 8] = [0; 8];
    u64_to_u8(&(size & u64::MAX), &mut length);
    for x in 0..8 {
        let chunk_position = ZERO_PADDING_MAX_SIZE_BYTES + x + 1;
        let length_position = x;
        chunk[chunk_position] = length[length_position];
    }
}

#[allow(dead_code)]
impl ChunkProcessor {
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        let mut data = data.as_ref();
        if !self.buffer.is_empty() {
            let size = data.len() + self.buffer.len();
            if size < CHUNK_SIZE_BYTES {
                self.buffer.extend_from_slice(data);
                return;
            }
            let (for_chunk, extra) = data.split_at(CHUNK_SIZE_BYTES - self.buffer.len());
            data = extra;
            self.buffer.extend_from_slice(for_chunk);
            log::debug!(
                "Buffer filled: length={0}, content={1:?}",
                self.buffer.len(),
                self.buffer
            );
            let chunk = &Chunk::try_from(self.buffer.drain(..).as_slice()).unwrap();
            self.state = self.state.process_chunk(chunk);
            self.size += CHUNK_LENGTH;
        }
        let chunks_iter = data.chunks_exact(CHUNK_SIZE_BYTES);
        self.buffer.extend_from_slice(chunks_iter.remainder());
        chunks_iter.for_each(|raw_chunk| {
            let chunk = &Chunk::try_from(raw_chunk).unwrap();
            self.state = self.state.process_chunk(chunk);
            self.size += CHUNK_LENGTH;
        });
    }

    pub fn finalize(mut self) -> Hash {
        let buffer_length = self.buffer.len();
        let size = self.size + (buffer_length as u64 * 8);
        self.buffer.push(INITIAL_BIT);
        self.buffer
            .append(&mut vec![0_u8; CHUNK_SIZE_BYTES - buffer_length - 1]);
        if buffer_length > ZERO_PADDING_MAX_SIZE_BYTES {
            log::debug!(
                "Buffer cannot hold padding: length={0}, content={1:?}",
                self.buffer.len(),
                self.buffer
            );
            let chunk = &Chunk::try_from(self.buffer.as_slice()).unwrap();
            self.state = self.state.process_chunk(chunk);
            self.buffer.clear();
            self.buffer.append(&mut vec![0_u8; CHUNK_SIZE_BYTES]);
        }
        log::debug!(
            "Buffer remaining: length={0}, content={1:?}",
            self.buffer.len(),
            self.buffer
        );
        let chunk = &mut Chunk::try_from(self.buffer.as_slice()).unwrap();
        chunk[buffer_length] = INITIAL_BIT;
        write_length(chunk, size);
        self.state = self.state.process_chunk(chunk);
        Hash::from(self.state.to_raw())
    }
}

#[cfg(test)]
mod test {
    use super::ChunkProcessor;
    use rstest::rstest;

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
    // Values chosen to fill a buffer that cannot hold the padding.
    #[case(
        "123456789012345678901234567890123456789012345678901234567890",
        "c5b549377c826cc3712418b064fc417e"
    )]
    #[case(
        "1234567890123456789012345678901234567890123456789012345678901",
        "931844f87f22a0ac1b7167979c8bea99"
    )]
    #[case(
        "12345678901234567890123456789012345678901234567890123456789012",
        "a29fba1f76305e4754853afb94525918"
    )]
    #[case(
        "123456789012345678901234567890123456789012345678901234567890123",
        "c3eb67ece68488bb394241d4f6a54244"
    )]
    #[case(
        "1234567890123456789012345678901234567890123456789012345678901234",
        "eb6c4179c0a7c82cc2828c1e6338e165"
    )]
    fn test_hash_rfc_examples(#[case] data: &str, #[case] expected: &str) {
        let digest = {
            let mut processor = ChunkProcessor::default();
            processor.update(data.as_bytes());
            processor.finalize()
        };
        let result = format!("{}", digest);
        assert_eq!(result, expected);
    }

    #[rustfmt::skip]
    #[rstest]
    #[case("12345678901234567890123456789012345678901234567890123456789012", "1", "ae6d673a00a645f542eade86ebdcbcc4")]
    #[case("12345678901234567890123456789012345678901234567890123456789012", "12", "7514243fcedea53164a3b6d212fd720b")]
    #[case("12345678901234567890123456789012345678901234567890123456789012", "123", "353e895e9d0a7bb2151c1f0120f036d2")]
    fn test_buffer_filled(#[case] left: &str, #[case] right: &str, #[case] expected: &str) {
        let digest = {
            let mut processor = ChunkProcessor::default();
            processor.update(left.as_bytes());
            processor.update(right.as_bytes());
            processor.finalize()
        };
        let result = format!("{}", digest);
        assert_eq!(result, expected);
    }
}
