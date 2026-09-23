//! Turns a pipe's bytes into text without ever splitting a character.
//!
//! A streaming reader hands back whatever happened to be in the pipe, which
//! cuts multi-byte characters in half at arbitrary boundaries. `read_to_string`
//! avoids the problem by reading everything first; a run that shows its output
//! as it arrives cannot, so it carries the incomplete tail to the next read.

/// Splits the longest valid UTF-8 prefix off `bytes`, leaving an incomplete
/// trailing character behind for the next read.
///
/// Genuinely invalid bytes -- as opposed to a character the pipe has not
/// finished delivering -- become a replacement character and are consumed, so
/// a binary blob cannot wedge the decoder.
pub fn take_decodable(bytes: &mut Vec<u8>) -> String {
    let mut text = String::new();

    loop {
        match std::str::from_utf8(bytes) {
            Ok(valid) => {
                text.push_str(valid);
                bytes.clear();
                return text;
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                if let Ok(valid) = std::str::from_utf8(&bytes[..valid_up_to]) {
                    text.push_str(valid);
                }

                match error.error_len() {
                    // Invalid, not incomplete: emit a replacement and go on.
                    Some(invalid) => {
                        text.push(char::REPLACEMENT_CHARACTER);
                        bytes.drain(..valid_up_to + invalid);
                    }
                    // The pipe cut a character in half. Keep it for next time.
                    None => {
                        bytes.drain(..valid_up_to);
                        return text;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::take_decodable;

    #[test]
    fn valid_bytes_decode_whole() {
        let mut bytes = b"hello".to_vec();

        assert_eq!(take_decodable(&mut bytes), "hello");
        assert!(bytes.is_empty());
    }

    #[test]
    fn an_incomplete_character_waits_for_the_next_read() {
        // "é" is 0xC3 0xA9; the pipe delivered only the first byte.
        let mut bytes = vec![b'a', 0xC3];

        assert_eq!(take_decodable(&mut bytes), "a");
        assert_eq!(bytes, vec![0xC3], "the half character is kept");

        bytes.push(0xA9);

        assert_eq!(take_decodable(&mut bytes), "é");
        assert!(bytes.is_empty());
    }

    #[test]
    fn invalid_bytes_become_a_replacement_and_are_consumed() {
        let mut bytes = vec![b'a', 0xFF, b'b'];

        assert_eq!(take_decodable(&mut bytes), "a\u{FFFD}b");
        assert!(bytes.is_empty());
    }

    #[test]
    fn a_character_split_across_three_reads_survives() {
        // "€" is 0xE2 0x82 0xAC.
        let mut bytes = vec![0xE2];
        assert_eq!(take_decodable(&mut bytes), "");

        bytes.push(0x82);
        assert_eq!(take_decodable(&mut bytes), "");

        bytes.push(0xAC);
        assert_eq!(take_decodable(&mut bytes), "€");
    }

    #[test]
    fn nothing_to_decode_yields_nothing() {
        let mut bytes = Vec::new();

        assert_eq!(take_decodable(&mut bytes), "");
    }
}
