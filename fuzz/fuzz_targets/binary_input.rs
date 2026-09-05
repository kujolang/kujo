const HEX_PREFIX: &[u8] = b"hex:";

pub fn raw_or_hex(data: &[u8], maximum: usize) -> Vec<u8> {
    let bounded = &data[..data.len().min(maximum)];
    if !bounded.starts_with(HEX_PREFIX) {
        return bounded.to_vec();
    }
    let encoded = &bounded[HEX_PREFIX.len()..];
    if encoded.len() % 2 != 0 {
        return bounded.to_vec();
    }
    let mut output = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.chunks_exact(2) {
        let Some(high) = hex_digit(pair[0]) else {
            return bounded.to_vec();
        };
        let Some(low) = hex_digit(pair[1]) else {
            return bounded.to_vec();
        };
        output.push((high << 4) | low);
    }
    output
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
