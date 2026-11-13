//! Path of Building import codes: URL-safe base64 of zlib-compressed XML.
//! The code is untrusted input (pasted or downloaded), so both the code and the inflated XML are size-capped.

use std::io::Read;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// Real builds are well under 1 MB of code; 2 MB leaves room for huge item/notes sections.
const MAX_CODE_BYTES: usize = 2 << 20;
/// Largest XML we accept after inflating (a zip bomb compresses 1000:1).
const MAX_XML_BYTES: u64 = 32 << 20;

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    CodeTooLarge,
    NotBase64,
    NotCompressed,
    XmlTooLarge,
    NotUtf8,
}

pub fn decode(code: &str) -> Result<String, DecodeError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(DecodeError::Empty);
    }
    if code.len() > MAX_CODE_BYTES {
        return Err(DecodeError::CodeTooLarge);
    }
    // URL-safe alphabet back to standard; PoB codes can come without padding or with line breaks
    let mut b64: String = code.chars().filter(|c| !c.is_whitespace()).map(|c| match c {
        '-' => '+',
        '_' => '/',
        c => c,
    }).collect();
    b64.truncate(b64.trim_end_matches('=').len());
    b64.extend(std::iter::repeat_n('=', (4 - b64.len() % 4) % 4));
    let packed = STANDARD.decode(b64).map_err(|_| DecodeError::NotBase64)?;

    let mut xml = Vec::new();
    flate2::read::ZlibDecoder::new(packed.as_slice())
        .take(MAX_XML_BYTES + 1)
        .read_to_end(&mut xml)
        .map_err(|_| DecodeError::NotCompressed)?;
    if xml.len() as u64 > MAX_XML_BYTES {
        return Err(DecodeError::XmlTooLarge);
    }
    String::from_utf8(xml).map_err(|_| DecodeError::NotUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    fn encode(xml: &[u8]) -> String {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(xml).unwrap();
        STANDARD.encode(e.finish().unwrap()).replace('+', "-").replace('/', "_")
    }

    #[test]
    fn roundtrips_with_urlsafe_alphabet_padding_and_whitespace() {
        // '?' bytes make the encoder emit both '-' and '_' substitutions
        let xml = "<PathOfBuilding><Build level=\"90\" className=\"Ranger\"/>\u{e9}?>?>?></PathOfBuilding>";
        let code = encode(xml.as_bytes());
        assert_eq!(decode(&code).unwrap(), xml);
        assert_eq!(decode(code.trim_end_matches('=')).unwrap(), xml);
        assert_eq!(decode(&format!("  {}\r\n", code)).unwrap(), xml);
        let split = format!("{}\n{}", &code[..10], &code[10..]);
        assert_eq!(decode(&split).unwrap(), xml);
    }

    #[test]
    fn rejects_bad_input_without_panicking() {
        assert_eq!(decode(""), Err(DecodeError::Empty));
        assert_eq!(decode("   "), Err(DecodeError::Empty));
        assert_eq!(decode("not base64 !!!"), Err(DecodeError::NotBase64));
        assert_eq!(decode("aGVsbG8gd29ybGQ"), Err(DecodeError::NotCompressed)); // valid base64, not zlib
        assert_eq!(decode(&"A".repeat(MAX_CODE_BYTES + 1)), Err(DecodeError::CodeTooLarge));
        assert_eq!(decode(&encode(&[0xFF, 0xFE, 0x00])), Err(DecodeError::NotUtf8));
    }

    #[test]
    fn zip_bomb_is_stopped_at_the_cap() {
        let bomb = encode(&vec![b'a'; (MAX_XML_BYTES + 1) as usize]);
        assert!(bomb.len() < MAX_CODE_BYTES, "bomb must fit the code cap to exercise the inflate cap");
        assert_eq!(decode(&bomb), Err(DecodeError::XmlTooLarge));
    }
}
