//! One captured stream, bounded at the head.
//!
//! Owns the size limit and the truncation flag. It does not own where the text
//! came from: the drain threads in `run` decode bytes into `&str` and this
//! decides how much of it is kept.

/// Text captured from one stream, stopping at a byte limit.
///
/// The head is kept rather than the tail. A command that fails says why near
/// the start -- the missing file, the unknown flag -- and a reader who has
/// already lost the middle is better served by the beginning than by the last
/// screen of a progress bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputBuffer {
    text:      String,
    limit:     usize,
    truncated: bool,
}

impl OutputBuffer {
    pub fn new(limit: usize) -> Self {
        Self { text: String::new(),
               limit,
               truncated: false }
    }

    /// Appends what fits, reporting whether anything observable changed.
    ///
    /// The return value drives the run's dirty flag, so it says "a frame would
    /// draw this differently", not "bytes arrived": the first append past the
    /// limit changes the truncation mark and counts, every one after it
    /// changes nothing and does not.
    pub fn append(&mut self, chunk: &str) -> bool {
        if self.truncated || chunk.is_empty() {
            return false;
        }

        let room = self.limit.saturating_sub(self.text.len());

        if chunk.len() <= room {
            self.text.push_str(chunk);
            return true;
        }

        self.text
            .push_str(&chunk[..floor_char_boundary(chunk, room)]);
        self.truncated = true;
        true
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether output was discarded because the limit was reached.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// The largest index at or below `at` that starts a character.
///
/// `str::floor_char_boundary` is unstable; this is the same rule. Cutting a
/// chunk at an arbitrary byte would panic on a multi-byte character, and the
/// limit is a round number of bytes that lands mid-character sooner or later.
fn floor_char_boundary(text: &str, at: usize) -> usize {
    if at >= text.len() {
        return text.len();
    }

    let mut index = at;
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }

    index
}

#[cfg(test)]
mod tests {
    use super::{OutputBuffer, floor_char_boundary};

    #[test]
    fn appending_within_the_limit_keeps_everything() {
        let mut buffer = OutputBuffer::new(64);

        assert!(buffer.append("hello "));
        assert!(buffer.append("world"));

        assert_eq!(buffer.text(), "hello world");
        assert!(!buffer.is_truncated());
    }

    #[test]
    fn appending_past_the_limit_keeps_the_head_and_marks_truncated() {
        let mut buffer = OutputBuffer::new(5);

        assert!(buffer.append("abc"));
        assert!(buffer.append("defgh"));

        assert_eq!(buffer.text(), "abcde");
        assert!(buffer.is_truncated());
    }

    #[test]
    fn a_truncated_buffer_reports_no_further_change() {
        let mut buffer = OutputBuffer::new(2);

        assert!(buffer.append("abcd"), "the truncation mark is a change");
        assert!(!buffer.append("efgh"), "nothing a frame would draw changed");

        assert_eq!(buffer.text(), "ab");
    }

    #[test]
    fn truncation_does_not_split_a_character() {
        // Each `é` is two bytes, so a four-byte limit falls inside the third.
        let mut buffer = OutputBuffer::new(5);

        buffer.append("ééé");

        assert_eq!(buffer.text(), "éé");
        assert!(buffer.is_truncated());
    }

    #[test]
    fn a_zero_limit_truncates_on_first_append() {
        let mut buffer = OutputBuffer::new(0);

        assert!(buffer.append("anything"));

        assert!(buffer.is_empty());
        assert!(buffer.is_truncated());
    }

    #[test]
    fn an_empty_append_is_not_a_change() {
        let mut buffer = OutputBuffer::new(8);

        assert!(!buffer.append(""));
        assert!(!buffer.is_truncated());
    }

    #[test]
    fn char_boundary_walks_back_to_a_character_start() {
        assert_eq!(floor_char_boundary("éé", 3), 2);
        assert_eq!(floor_char_boundary("éé", 4), 4);
        assert_eq!(floor_char_boundary("abc", 99), 3);
        assert_eq!(floor_char_boundary("é", 1), 0);
    }
}
