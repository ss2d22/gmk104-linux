pub fn parse_hex_u16(s: &str) -> Result<u16, String> {
    let s = s.trim().to_lowercase();
    let s = s.strip_prefix("0x").unwrap_or(&s);
    u16::from_str_radix(s, 16).map_err(|e| format!("Invalid hex value '{}': {}", s, e))
}

pub fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn format_hex_dump(bytes: &[u8], offset: usize) -> String {
    let mut result = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let addr = offset + i * 16;
        result.push_str(&format!("{:04x}: ", addr));

        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                result.push(' ');
            }
            result.push_str(&format!("{:02x} ", byte));
        }

        let padding = 16 - chunk.len();
        for _ in 0..padding {
            result.push_str("   ");
        }
        if chunk.len() <= 8 {
            result.push(' ');
        }

        result.push_str(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                result.push(*byte as char);
            } else {
                result.push('.');
            }
        }
        result.push_str("|\n");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_u16() {
        assert_eq!(parse_hex_u16("0x320f").unwrap(), 0x320f);
        assert_eq!(parse_hex_u16("320f").unwrap(), 0x320f);
        assert_eq!(parse_hex_u16("0X320F").unwrap(), 0x320f);
        assert_eq!(parse_hex_u16("ffff").unwrap(), 0xffff);
        assert_eq!(parse_hex_u16("0").unwrap(), 0);
        assert!(parse_hex_u16("gggg").is_err());
        assert!(parse_hex_u16("10000").is_err());
    }

    #[test]
    fn test_format_hex() {
        assert_eq!(format_hex(&[0x01, 0x02, 0x03]), "01 02 03");
        assert_eq!(format_hex(&[0xff, 0x00]), "ff 00");
        assert_eq!(format_hex(&[]), "");
    }
}
