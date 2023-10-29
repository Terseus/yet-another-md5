use std::fmt::Display;
use std::io::Cursor;
use std::io::Read;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use log::debug;
use log::trace;

const CHUNK_SIZE_BYTES: usize = 64; // 512 / 8
const BLOCK_SIZE_WORDS: usize = CHUNK_SIZE_BYTES / 4;
const INITIAL_BIT_SIZE_BYTES: usize = 1;
const INITIAL_BIT: u8 = 0x80; // 1 in big endian.
const LENGTH_SIZE_BYTES: usize = 8; // 64 / 8
const ZERO_PADDING_MAX_SIZE_BYTES: usize =
    CHUNK_SIZE_BYTES - LENGTH_SIZE_BYTES - INITIAL_BIT_SIZE_BYTES;
const INITIAL_WORD_A: u32 = 0x67452301;
const INITIAL_WORD_B: u32 = 0xefcdab89;
const INITIAL_WORD_C: u32 = 0x98badcfe;
const INITIAL_WORD_D: u32 = 0x10325476;
// Precomputed table for T[i] = floor(2^32 * abs(sin(i))) for i = 1..64
const SINE_TABLE: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[derive(Debug)]
pub struct Hash {
    value: [u8; 16],
}

impl From<[u8; 16]> for Hash {
    fn from(value: [u8; 16]) -> Hash {
        Hash { value }
    }
}

impl Display for Hash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for value in self.value.iter() {
            write!(formatter, "{:02x}", value)?
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Chunk([u8; CHUNK_SIZE_BYTES]);

impl From<[u8; CHUNK_SIZE_BYTES]> for Chunk {
    fn from(value: [u8; CHUNK_SIZE_BYTES]) -> Self {
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

impl Chunk {
    pub fn empty() -> Self {
        Chunk([0; CHUNK_SIZE_BYTES])
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PaddingState {
    InitialBit,
    Length,
    Done,
}

type Block = [u32; BLOCK_SIZE_WORDS];

struct ChunkProvider<'a> {
    input: &'a mut dyn Read,
    padding_state: PaddingState,
    size: u64,
}

impl<'a> ChunkProvider<'a> {
    pub fn new(input: &'a mut dyn Read) -> Self {
        ChunkProvider {
            input,
            padding_state: PaddingState::InitialBit,
            size: 0,
        }
    }

    fn write_length(&self, chunk: &mut Chunk) -> Result<()> {
        let mut length: [u8; 8] = [0; 8];
        u64_to_u8(&(self.size & u64::MAX), &mut length)?;
        for x in 0..8 {
            let chunk_position = ZERO_PADDING_MAX_SIZE_BYTES + x + 1;
            let length_position = x;
            trace!(
                "chunk[{:?}]({:?}) = length[{:?}]({:?})",
                chunk_position,
                chunk.0[chunk_position],
                length_position,
                length[length_position]
            );
            chunk.0[chunk_position] = length[length_position];
        }
        Ok(())
    }

    fn read(&mut self, buffer: &mut Chunk) -> Result<Option<()>> {
        match self.input.read(&mut buffer.0) {
            Err(error) => Err(anyhow!(error)),
            Ok(bytes_read) => {
                if bytes_read == 0 && self.padding_state == PaddingState::Done {
                    return Ok(None);
                }
                // Length is in bits
                self.size += u64::try_from(bytes_read * 8)?;
                debug!("bytes_read: {}", bytes_read);
                debug!("size: {:?}", self.size);
                trace!("buffer: {}", buffer);
                if bytes_read == 0 {
                    debug!("empty chunk readed");
                    debug!("current padding state: {:?}", self.padding_state);
                    buffer.0.fill(0);
                    match &self.padding_state {
                        PaddingState::InitialBit => {
                            buffer.0[0] = INITIAL_BIT;
                            self.write_length(buffer)?;
                            self.padding_state = PaddingState::Done;
                        }
                        PaddingState::Length => {
                            self.write_length(buffer)?;
                            self.padding_state = PaddingState::Done;
                        }
                        PaddingState::Done => {
                            return Ok(None);
                        }
                    }
                    trace!("chunk with padding: {}", buffer);
                    return Ok(Some(()));
                }
                if bytes_read < buffer.len() {
                    debug!("last chunk readed");
                    buffer.0[bytes_read] = INITIAL_BIT;
                    buffer.0[bytes_read + 1..].fill(0);
                    self.padding_state = PaddingState::Length;
                    if bytes_read <= ZERO_PADDING_MAX_SIZE_BYTES {
                        debug!("chunk can hold padding");
                        self.write_length(buffer)?;
                        trace!("buffer with padding: {}", buffer);
                        self.padding_state = PaddingState::Done;
                    }
                }
                Ok(Some(()))
            }
        }
    }
}

const fn aux_fun_f(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!(x) & z)
}

const fn aux_fun_g(x: u32, y: u32, z: u32) -> u32 {
    (x & z) | (y & !(z))
}

const fn aux_fun_h(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

const fn aux_fun_i(x: u32, y: u32, z: u32) -> u32 {
    y ^ (x | !(z))
}

pub struct Md5Hasher {
    state: HashComputeState,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct HashComputeState {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

impl Display for HashComputeState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "HashComputeState {{ a: {:0>8x}, b: {:0>8x}, c: {:0>8x}, d: {:0>8x} }}",
            self.a, self.b, self.c, self.d
        )?;
        Ok(())
    }
}

macro_rules! Md5Op {
    ($self:ident, $block:ident, $aux_fun:ident, $a:ident, $b:ident, $c:ident, $d:ident, $k:expr, $s:expr, $i:expr) => {
        HashComputeState {
            $a: {
                $self
                    .$a
                    .wrapping_add($aux_fun($self.$b, $self.$c, $self.$d))
                    .wrapping_add($block[$k])
                    .wrapping_add(SINE_TABLE[$i])
                    .rotate_left($s)
                    .wrapping_add($self.$b)
            },
            $b: $self.$b,
            $c: $self.$c,
            $d: $self.$d,
        }
    };
}

// TODO: Try a struct to be used as mutable, benchmark it and see the difference.
impl HashComputeState {
    pub fn new_initial() -> Self {
        HashComputeState {
            a: INITIAL_WORD_A,
            b: INITIAL_WORD_B,
            c: INITIAL_WORD_C,
            d: INITIAL_WORD_D,
        }
    }

    pub fn advance_step(self, block: &Block, step: u8) -> Self {
        match step {
            // Round 1
            1 => Md5Op!(self, block, aux_fun_f, a, b, c, d, 0, 7, 0), // [ABCD  0  7  1]
            2 => Md5Op!(self, block, aux_fun_f, d, a, b, c, 1, 12, 1), // [DABC  1 12  2]
            3 => Md5Op!(self, block, aux_fun_f, c, d, a, b, 2, 17, 2), // [CDAB  2 17  3]
            4 => Md5Op!(self, block, aux_fun_f, b, c, d, a, 3, 22, 3), // [BCDA  3 22  4]
            5 => Md5Op!(self, block, aux_fun_f, a, b, c, d, 4, 7, 4), // [ABCD  4  7  5]
            6 => Md5Op!(self, block, aux_fun_f, d, a, b, c, 5, 12, 5), // [DABC  5 12  6]
            7 => Md5Op!(self, block, aux_fun_f, c, d, a, b, 6, 17, 6), // [CDAB  6 17  7]
            8 => Md5Op!(self, block, aux_fun_f, b, c, d, a, 7, 22, 7), // [BCDA  7 22  8]
            9 => Md5Op!(self, block, aux_fun_f, a, b, c, d, 8, 7, 8), // [ABCD  8  7  9]
            10 => Md5Op!(self, block, aux_fun_f, d, a, b, c, 9, 12, 9), // [DABC  9 12 10]
            11 => Md5Op!(self, block, aux_fun_f, c, d, a, b, 10, 17, 10), // [CDAB 10 17 11]
            12 => Md5Op!(self, block, aux_fun_f, b, c, d, a, 11, 22, 11), // [BCDA 11 22 12]
            13 => Md5Op!(self, block, aux_fun_f, a, b, c, d, 12, 7, 12), // [ABCD 12  7 13]
            14 => Md5Op!(self, block, aux_fun_f, d, a, b, c, 13, 12, 13), // [DABC 13 12 14]
            15 => Md5Op!(self, block, aux_fun_f, c, d, a, b, 14, 17, 14), // [CDAB 14 17 15]
            16 => Md5Op!(self, block, aux_fun_f, b, c, d, a, 15, 22, 15), // [BCDA 15 22 16]
            // Round 2
            17 => Md5Op!(self, block, aux_fun_g, a, b, c, d, 1, 5, 16), // [ABCD  1  5 17]
            18 => Md5Op!(self, block, aux_fun_g, d, a, b, c, 6, 9, 17), // [DABC  6  9 18]
            19 => Md5Op!(self, block, aux_fun_g, c, d, a, b, 11, 14, 18), // [CDAB 11 14 19]
            20 => Md5Op!(self, block, aux_fun_g, b, c, d, a, 0, 20, 19), // [BCDA  0 20 20]
            21 => Md5Op!(self, block, aux_fun_g, a, b, c, d, 5, 5, 20), // [ABCD  5  5 21]
            22 => Md5Op!(self, block, aux_fun_g, d, a, b, c, 10, 9, 21), // [DABC 10  9 22]
            23 => Md5Op!(self, block, aux_fun_g, c, d, a, b, 15, 14, 22), // [CDAB 15 14 23]
            24 => Md5Op!(self, block, aux_fun_g, b, c, d, a, 4, 20, 23), // [BCDA  4 20 24]
            25 => Md5Op!(self, block, aux_fun_g, a, b, c, d, 9, 5, 24), // [ABCD  9  5 25]
            26 => Md5Op!(self, block, aux_fun_g, d, a, b, c, 14, 9, 25), // [DABC 14  9 26]
            27 => Md5Op!(self, block, aux_fun_g, c, d, a, b, 3, 14, 26), // [CDAB  3 14 27]
            28 => Md5Op!(self, block, aux_fun_g, b, c, d, a, 8, 20, 27), // [BCDA  8 20 28]
            29 => Md5Op!(self, block, aux_fun_g, a, b, c, d, 13, 5, 28), // [ABCD 13  5 29]
            30 => Md5Op!(self, block, aux_fun_g, d, a, b, c, 2, 9, 29), // [DABC  2  9 30]
            31 => Md5Op!(self, block, aux_fun_g, c, d, a, b, 7, 14, 30), // [CDAB  7 14 31]
            32 => Md5Op!(self, block, aux_fun_g, b, c, d, a, 12, 20, 31), // [BCDA 12 20 32]
            // Round 3
            33 => Md5Op!(self, block, aux_fun_h, a, b, c, d, 5, 4, 32), // [ABCD  5  4 33]
            34 => Md5Op!(self, block, aux_fun_h, d, a, b, c, 8, 11, 33), // [DABC  8 11 34]
            35 => Md5Op!(self, block, aux_fun_h, c, d, a, b, 11, 16, 34), // [CDAB 11 16 35]
            36 => Md5Op!(self, block, aux_fun_h, b, c, d, a, 14, 23, 35), // [BCDA 14 23 36]
            37 => Md5Op!(self, block, aux_fun_h, a, b, c, d, 1, 4, 36), // [ABCD  1  4 37]
            38 => Md5Op!(self, block, aux_fun_h, d, a, b, c, 4, 11, 37), // [DABC  4 11 38]
            39 => Md5Op!(self, block, aux_fun_h, c, d, a, b, 7, 16, 38), // [CDAB  7 16 39]
            40 => Md5Op!(self, block, aux_fun_h, b, c, d, a, 10, 23, 39), // [BCDA 10 23 40]
            41 => Md5Op!(self, block, aux_fun_h, a, b, c, d, 13, 4, 40), // [ABCD 13  4 41]
            42 => Md5Op!(self, block, aux_fun_h, d, a, b, c, 0, 11, 41), // [DABC  0 11 42]
            43 => Md5Op!(self, block, aux_fun_h, c, d, a, b, 3, 16, 42), // [CDAB  3 16 43]
            44 => Md5Op!(self, block, aux_fun_h, b, c, d, a, 6, 23, 43), // [BCDA  6 23 44]
            45 => Md5Op!(self, block, aux_fun_h, a, b, c, d, 9, 4, 44), // [ABCD  9  4 45]
            46 => Md5Op!(self, block, aux_fun_h, d, a, b, c, 12, 11, 45), // [DABC 12 11 46]
            47 => Md5Op!(self, block, aux_fun_h, c, d, a, b, 15, 16, 46), // [CDAB 15 16 47]
            48 => Md5Op!(self, block, aux_fun_h, b, c, d, a, 2, 23, 47), // [BCDA  2 23 48]
            // Round 4
            49 => Md5Op!(self, block, aux_fun_i, a, b, c, d, 0, 6, 48), // [ABCD  0  6 49]
            50 => Md5Op!(self, block, aux_fun_i, d, a, b, c, 7, 10, 49), // [DABC  7 10 50]
            51 => Md5Op!(self, block, aux_fun_i, c, d, a, b, 14, 15, 50), // [CDAB 14 15 51]
            52 => Md5Op!(self, block, aux_fun_i, b, c, d, a, 5, 21, 51), // [BCDA  5 21 52]
            53 => Md5Op!(self, block, aux_fun_i, a, b, c, d, 12, 6, 52), // [ABCD 12  6 53]
            54 => Md5Op!(self, block, aux_fun_i, d, a, b, c, 3, 10, 53), // [DABC  3 10 54]
            55 => Md5Op!(self, block, aux_fun_i, c, d, a, b, 10, 15, 54), // [CDAB 10 15 55]
            56 => Md5Op!(self, block, aux_fun_i, b, c, d, a, 1, 21, 55), // [BCDA  1 21 56]
            57 => Md5Op!(self, block, aux_fun_i, a, b, c, d, 8, 6, 56), // [ABCD  8  6 57]
            58 => Md5Op!(self, block, aux_fun_i, d, a, b, c, 15, 10, 57), // [DABC 15 10 58]
            59 => Md5Op!(self, block, aux_fun_i, c, d, a, b, 6, 15, 58), // [CDAB  6 15 59]
            60 => Md5Op!(self, block, aux_fun_i, b, c, d, a, 13, 21, 59), // [BCDA 13 21 60]
            61 => Md5Op!(self, block, aux_fun_i, a, b, c, d, 4, 6, 60), // [ABCD  4  6 61]
            62 => Md5Op!(self, block, aux_fun_i, d, a, b, c, 11, 10, 61), // [DABC 11 10 62]
            63 => Md5Op!(self, block, aux_fun_i, c, d, a, b, 2, 15, 62), // [CDAB  2 15 63]
            64 => Md5Op!(self, block, aux_fun_i, b, c, d, a, 9, 21, 63), // [BCDA  9 21 64]
            _ => unreachable!(),
        }
    }

    pub fn process_chunk(self, chunk: &Chunk) -> Result<Self> {
        let mut block: Block = [0; BLOCK_SIZE_WORDS];
        for (index, item) in block.iter_mut().enumerate() {
            *item = u8_to_u32(&chunk.0[(index * 4)..((index * 4) + 4)].try_into()?)?;
        }
        let mut result = self;
        for step in 1..65 {
            result = result.advance_step(&block, step);
            trace!("State at step {:0>2}: {}", step, result);
        }
        Ok(HashComputeState {
            a: self.a.wrapping_add(result.a),
            b: self.b.wrapping_add(result.b),
            c: self.c.wrapping_add(result.c),
            d: self.d.wrapping_add(result.d),
        })
    }
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

    pub fn hash(data: Vec<u8>) -> Result<[u8; 16]> {
        let mut cursor = Cursor::new(data);
        let mut chunk_provider = ChunkProvider::new(&mut cursor);
        let mut hasher = Md5Hasher::new();
        let mut buffer = Chunk::empty();
        while (chunk_provider.read(&mut buffer)?).is_some() {
            hasher.add_raw_chunk(buffer)?;
        }
        hasher.compute()
    }

    pub fn add_chunk(&mut self, chunk: [u8; CHUNK_SIZE_BYTES]) -> Result<()> {
        self.add_raw_chunk(Chunk::from(chunk))
    }

    pub fn compute(&self) -> Result<[u8; 16]> {
        let mut buffer: [u8; 16] = [0; 16];
        buffer[0..4].copy_from_slice(&self.state_var_to_u8(&self.state.a)?);
        buffer[4..8].copy_from_slice(&self.state_var_to_u8(&self.state.b)?);
        buffer[8..12].copy_from_slice(&self.state_var_to_u8(&self.state.c)?);
        buffer[12..16].copy_from_slice(&self.state_var_to_u8(&self.state.d)?);
        Ok(buffer)
    }

    fn add_raw_chunk(&mut self, chunk: Chunk) -> Result<()> {
        self.state = self.state.process_chunk(&chunk)?;
        Ok(())
    }

    fn state_var_to_u8(&self, state_var: &u32) -> Result<[u8; 4]> {
        let mut buffer: [u8; 4] = [0; 4];
        for (index, item) in buffer.iter_mut().enumerate().take(4) {
            *item = u8::try_from((0xff << (index * 8) & state_var) >> (index * 8))?;
        }
        Ok(buffer)
    }
}

fn u64_to_u8(source: &u64, buffer: &mut [u8; 8]) -> Result<()> {
    for (index, item) in buffer.iter_mut().enumerate().take(8) {
        *item = u8::try_from((0xff << (index * 8) & source) >> (index * 8)).with_context(|| {
            anyhow!("Error transforming byte {:?} in source {:?}", index, source)
        })?;
    }
    Ok(())
}

fn u8_to_u32(source: &[u8; 4]) -> Result<u32> {
    let mut result = 0u32;
    for (index, item) in source.iter().enumerate().take(4) {
        result |= (*item as u32) << (index * 8);
    }
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use log::LevelFilter;
    use rstest::*;
    use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};

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
    #[case(0xffffffffffffffff, [0xff; 8])]
    #[case(0xffffffff00000000, [0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff])]
    #[case(0x0123456789abcdef, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01])]
    fn test_u64_to_u8(#[case] input: u64, #[case] expected: [u8; 8]) {
        let mut octets: [u8; 8] = [0; 8];
        u64_to_u8(&input, &mut octets).expect("Error decoding in octets");
        assert_eq!(&octets, &expected);
    }

    #[rstest]
    #[case([0xff; 4], 0xffffffff)]
    #[case([0, 0, 0xff, 0xff], 0xffff0000)]
    #[case([0x67, 0x45, 0x23, 0x01], 0x01234567)]
    fn test_u8_to_u32(#[case] input: [u8; 4], #[case] expected: u32) {
        let result = u8_to_u32(&input).expect("Error combining octets");
        assert_eq!(result, expected);
    }

    // Inputs and outputs taken from https://rosettacode.org/wiki/MD5/Implementation_Debug
    #[rstest]
    #[case(
        vec![],
        vec![
            Chunk::from([
                0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        ]
    )]
    #[case(
        vec![0x61],
        vec![
            Chunk::from([
                0x61, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        ]
    )]
    #[case(
        vec![0x61, 0x62, 0x63],
        vec![
            Chunk::from([
                0x61, 0x62, 0x63, 0x80, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        ]
    )]
    #[case(
        vec![
            0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x20,
            0x64, 0x69, 0x67, 0x65, 0x73, 0x74,
        ],
        vec![
            Chunk::from([
                0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x20,
                0x64, 0x69, 0x67, 0x65, 0x73, 0x74, 0x80, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        ]
    )]
    #[case(
        vec![
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
            0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
            0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
            0x59, 0x5A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
            0x77, 0x78, 0x79, 0x7A, 0x30, 0x31, 0x32, 0x33,
            0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
        ],
        vec![
            Chunk::from([
                0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
                0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
                0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
                0x59, 0x5A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                0x77, 0x78, 0x79, 0x7A, 0x30, 0x31, 0x32, 0x33,
                0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x80, 0x00,
            ]),
            Chunk::from([
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0xf0, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        ]
    )]
    #[case(
        vec![
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
            0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34,
            0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32,
            0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
            0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34,
            0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32,
            0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
        ],
        vec![
            Chunk::from([
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
                0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
                0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34,
                0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32,
                0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
                0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
                0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34,
            ]),
            Chunk::from([
                0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32,
                0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x80, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        ]
    )]
    fn test_padding(#[case] contents: Vec<u8>, #[case] expected: Vec<Chunk>) {
        let mut result: Vec<Chunk> = vec![];
        let mut buffer = Chunk::empty();
        let mut cursor = Cursor::new(contents);
        let mut chunk_provider = ChunkProvider::new(&mut cursor);
        while let Some(_) = chunk_provider.read(&mut buffer).unwrap() {
            result.push(buffer.clone());
        }
        assert_eq!(result, expected);
    }

    // Example values taken from https://rosettacode.org/wiki/MD5/Implementation_Debug
    #[rstest]
    #[case(1, HashComputeState {a: 0xa5202774, b: 0xefcdab89, c: 0x98badcfe, d: 0x10325476})]
    #[case(2, HashComputeState {a: 0xa5202774, b: 0xefcdab89, c: 0x98badcfe, d: 0xf59592dd})]
    #[case(3, HashComputeState {a: 0xa5202774, b: 0xefcdab89, c: 0xe7f06b23, d: 0xf59592dd})]
    #[case(4, HashComputeState {a: 0xa5202774, b: 0x1b163203, c: 0xe7f06b23, d: 0xf59592dd})]
    #[case(5, HashComputeState {a: 0x32033344, b: 0x1b163203, c: 0xe7f06b23, d: 0xf59592dd})]
    #[case(6, HashComputeState {a: 0x32033344, b: 0x1b163203, c: 0xe7f06b23, d: 0x2f35d494})]
    #[case(7, HashComputeState {a: 0x32033344, b: 0x1b163203, c: 0xf5b158db, d: 0x2f35d494})]
    #[case(8, HashComputeState {a: 0x32033344, b: 0x9bc13ce9, c: 0xf5b158db, d: 0x2f35d494})]
    #[case(9, HashComputeState {a: 0x3893b991, b: 0x9bc13ce9, c: 0xf5b158db, d: 0x2f35d494})]
    #[case(10, HashComputeState {a: 0x3893b991, b: 0x9bc13ce9, c: 0xf5b158db, d: 0xfce4a312})]
    #[case(11, HashComputeState {a: 0x3893b991, b: 0x9bc13ce9, c: 0xe1ef0576, d: 0xfce4a312})]
    #[case(12, HashComputeState {a: 0x3893b991, b: 0x70768a29, c: 0xe1ef0576, d: 0xfce4a312})]
    #[case(13, HashComputeState {a: 0xf56c7cf1, b: 0x70768a29, c: 0xe1ef0576, d: 0xfce4a312})]
    #[case(14, HashComputeState {a: 0xf56c7cf1, b: 0x70768a29, c: 0xe1ef0576, d: 0x374943a7})]
    #[case(15, HashComputeState {a: 0xf56c7cf1, b: 0x70768a29, c: 0x5aa53f75, d: 0x374943a7})]
    #[case(16, HashComputeState {a: 0xf56c7cf1, b: 0xd6819c6a, c: 0x5aa53f75, d: 0x374943a7})]
    #[case(20, HashComputeState {a: 0x1c7d7513, b: 0xbd782e17, c: 0xc095f13a, d: 0x7bd57a3a})]
    #[case(24, HashComputeState {a: 0x3d1e3e6c, b: 0xe422531a, c: 0xeb41643e, d: 0x68b7b3e3})]
    #[case(64, HashComputeState {a: 0x7246fad3, b: 0x14e45506, c: 0xff4ea3eb, d: 0x6e10a476})]
    fn test_compute_state_advance_empty_string(
        #[case] steps: u8,
        #[case] expected: HashComputeState,
    ) {
        #[rustfmt::skip]
        let block: [u32; BLOCK_SIZE_WORDS] = [
            0x00000080, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000,
        ];
        let mut instance = HashComputeState::new_initial();
        for x in 1..(steps + 1) {
            instance = instance.advance_step(&block, x);
        }
        assert_eq!(instance, expected);
    }

    #[rstest]
    #[case(
        Chunk::from([
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]),
        HashComputeState {a: 0xd98c1dd4, b: 0x04b2008f, c: 0x980980e9, d: 0x7e42f8ec}
    )]
    fn test_process_chunk(#[case] chunk: Chunk, #[case] expected: HashComputeState) {
        let mut instance = HashComputeState::new_initial();
        instance = instance
            .process_chunk(&chunk)
            .expect("Error processing chunk");
        assert_eq!(instance, expected);
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
    fn test_compute_single_chunk(#[case] chunk: [u8; CHUNK_SIZE_BYTES], #[case] expected: &str) {
        let mut instance = Md5Hasher::new();
        instance.add_chunk(chunk).expect("Error adding chunk");
        let digest = Hash::from(instance.compute().expect("Error in compute"));
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
    fn test_hash(#[case] data: &str, #[case] expected: &str) {
        let data = Vec::from(data.as_bytes());
        let digest = Hash::from(Md5Hasher::hash(data).expect("Error in hash"));
        let result = format!("{}", digest);
        assert_eq!(result, expected);
    }
}
