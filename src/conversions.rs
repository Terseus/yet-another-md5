pub fn extract_u8(index: &usize, value: &u64) -> u8 {
    match u8::try_from((0xff << (index * 8) & value) >> (index * 8)) {
        Ok(value) => value,
        Err(_) => panic!(
            "Error extracting u8 from u64; index={:?}, u64={:?}",
            index, value
        ),
    }
}

pub fn u64_to_u8(source: &u64, buffer: &mut [u8; 8]) {
    for (index, item) in buffer.iter_mut().enumerate().take(8) {
        *item = extract_u8(&index, source);
    }
}

pub fn u8_to_u32(source: &[u8; 4]) -> u32 {
    let mut result = 0u32;
    for (index, item) in source.iter().enumerate().take(4) {
        result |= (*item as u32) << (index * 8);
    }
    result
}

pub fn u32_to_u8(source: &u32) -> [u8; 4] {
    let mut buffer: [u8; 4] = [0; 4];
    for (index, item) in buffer.iter_mut().enumerate().take(4) {
        *item = extract_u8(&index, &(*source as u64));
    }
    buffer
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0xffffffffffffffff, [0xff; 8])]
    #[case(0xffffffff00000000, [0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff])]
    #[case(0x0123456789abcdef, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01])]
    fn test_u64_to_u8(#[case] input: u64, #[case] expected: [u8; 8]) {
        let mut octets: [u8; 8] = [0; 8];
        u64_to_u8(&input, &mut octets);
        assert_eq!(&octets, &expected);
    }

    #[rstest]
    #[case([0xff; 4], 0xffffffff)]
    #[case([0, 0, 0xff, 0xff], 0xffff0000)]
    #[case([0x67, 0x45, 0x23, 0x01], 0x01234567)]
    fn test_u8_to_u32(#[case] input: [u8; 4], #[case] expected: u32) {
        let result = u8_to_u32(&input);
        assert_eq!(result, expected);
    }
}
