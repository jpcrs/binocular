pub fn try_decode_utf16(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 {
        return None;
    }

    if bytes[0] == 0xFF && bytes[1] == 0xFE {
        return Some(decode_utf16(bytes, Endian::Little));
    }

    if bytes[0] == 0xFE && bytes[1] == 0xFF {
        return Some(decode_utf16(bytes, Endian::Big));
    }

    None
}

enum Endian {
    Little,
    Big,
}

fn decode_utf16(bytes: &[u8], endian: Endian) -> String {
    let u16_iter = bytes[2..].chunks_exact(2).map(|chunk| match endian {
        Endian::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
        Endian::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
    });

    char::decode_utf16(u16_iter)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}
