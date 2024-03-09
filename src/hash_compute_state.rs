use crate::chunk::Chunk;
use crate::chunk::CHUNK_SIZE_BYTES;
use crate::conversions::u32_to_u8;
use crate::conversions::u8_to_u32;

use log::trace;
use std::fmt::Display;

const BLOCK_SIZE_WORDS: usize = CHUNK_SIZE_BYTES / 4;
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
const INITIAL_WORD_A: u32 = 0x67452301;
const INITIAL_WORD_B: u32 = 0xefcdab89;
const INITIAL_WORD_C: u32 = 0x98badcfe;
const INITIAL_WORD_D: u32 = 0x10325476;

type Block = [u32; BLOCK_SIZE_WORDS];

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HashComputeState {
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

impl Default for HashComputeState {
    fn default() -> Self {
        HashComputeState {
            a: INITIAL_WORD_A,
            b: INITIAL_WORD_B,
            c: INITIAL_WORD_C,
            d: INITIAL_WORD_D,
        }
    }
}

impl HashComputeState {
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

    pub fn process_chunk(self, chunk: &Chunk) -> Self {
        let mut block: Block = [0; BLOCK_SIZE_WORDS];
        for (index, item) in block.iter_mut().enumerate() {
            let unpacked: [u8; 4] = match chunk.0[(index * 4)..((index * 4) + 4)].try_into() {
                Ok(value) => value,
                Err(_) => panic!(
                    "process_chunk: error extracting word; position={:?}, chunk={:?}",
                    index, chunk
                ),
            };
            *item = u8_to_u32(&unpacked);
        }
        let mut result = self;
        for step in 1..65 {
            result = result.advance_step(&block, step);
            trace!("State at step {:0>2}: {}", step, result);
        }
        HashComputeState {
            a: self.a.wrapping_add(result.a),
            b: self.b.wrapping_add(result.b),
            c: self.c.wrapping_add(result.c),
            d: self.d.wrapping_add(result.d),
        }
    }

    pub fn to_raw(self) -> [u8; 16] {
        let mut buffer: [u8; 16] = [0; 16];
        buffer[0..4].copy_from_slice(&u32_to_u8(&self.a));
        buffer[4..8].copy_from_slice(&u32_to_u8(&self.b));
        buffer[8..12].copy_from_slice(&u32_to_u8(&self.c));
        buffer[12..16].copy_from_slice(&u32_to_u8(&self.d));
        buffer
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    // Example values taken from https://rosettacode.org/wiki/MD5/Implementation_Debug
    #[rstest]
    #[case(1, HashComputeState{a: 0xa5202774, b: 0xefcdab89, c: 0x98badcfe, d: 0x10325476})]
    #[case(2, HashComputeState{a: 0xa5202774, b: 0xefcdab89, c: 0x98badcfe, d: 0xf59592dd})]
    #[case(3, HashComputeState{a: 0xa5202774, b: 0xefcdab89, c: 0xe7f06b23, d: 0xf59592dd})]
    #[case(4, HashComputeState{a: 0xa5202774, b: 0x1b163203, c: 0xe7f06b23, d: 0xf59592dd})]
    #[case(5, HashComputeState{a: 0x32033344, b: 0x1b163203, c: 0xe7f06b23, d: 0xf59592dd})]
    #[case(6, HashComputeState{a: 0x32033344, b: 0x1b163203, c: 0xe7f06b23, d: 0x2f35d494})]
    #[case(7, HashComputeState{a: 0x32033344, b: 0x1b163203, c: 0xf5b158db, d: 0x2f35d494})]
    #[case(8, HashComputeState{a: 0x32033344, b: 0x9bc13ce9, c: 0xf5b158db, d: 0x2f35d494})]
    #[case(9, HashComputeState{a: 0x3893b991, b: 0x9bc13ce9, c: 0xf5b158db, d: 0x2f35d494})]
    #[case(10, HashComputeState{a: 0x3893b991, b: 0x9bc13ce9, c: 0xf5b158db, d: 0xfce4a312})]
    #[case(11, HashComputeState{a: 0x3893b991, b: 0x9bc13ce9, c: 0xe1ef0576, d: 0xfce4a312})]
    #[case(12, HashComputeState{a: 0x3893b991, b: 0x70768a29, c: 0xe1ef0576, d: 0xfce4a312})]
    #[case(13, HashComputeState{a: 0xf56c7cf1, b: 0x70768a29, c: 0xe1ef0576, d: 0xfce4a312})]
    #[case(14, HashComputeState{a: 0xf56c7cf1, b: 0x70768a29, c: 0xe1ef0576, d: 0x374943a7})]
    #[case(15, HashComputeState{a: 0xf56c7cf1, b: 0x70768a29, c: 0x5aa53f75, d: 0x374943a7})]
    #[case(16, HashComputeState{a: 0xf56c7cf1, b: 0xd6819c6a, c: 0x5aa53f75, d: 0x374943a7})]
    #[case(20, HashComputeState{a: 0x1c7d7513, b: 0xbd782e17, c: 0xc095f13a, d: 0x7bd57a3a})]
    #[case(24, HashComputeState{a: 0x3d1e3e6c, b: 0xe422531a, c: 0xeb41643e, d: 0x68b7b3e3})]
    #[case(64, HashComputeState{a: 0x7246fad3, b: 0x14e45506, c: 0xff4ea3eb, d: 0x6e10a476})]
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
        let mut instance = HashComputeState::default();
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
        HashComputeState{a: 0xd98c1dd4, b: 0x04b2008f, c: 0x980980e9, d: 0x7e42f8ec}
    )]
    fn test_process_chunk(#[case] chunk: Chunk, #[case] expected: HashComputeState) {
        let mut instance = HashComputeState::default();
        instance = instance.process_chunk(&chunk);
        assert_eq!(instance, expected);
    }
}
