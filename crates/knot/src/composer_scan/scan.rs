//! The vocabulary of composer constructs, and the two entry points that
//! produce them.

use std::ops::Range;

use crate::composer_scan::dirty;
use crate::composer_scan::markdown;
use crate::composer_scan::tokens;

/// What a run of the buffer is.
///
/// A closed set, and an enum rather than a string for the reason
/// `.claude/rules/rust-structure.md` gives: a `_ => prose` arm turns an
/// unrecognised construct into an indistinguishable default.
///
/// The three decoration collections the composer builds group these:
/// [`Construct::Attachment`] alone, the two token variants, and all the
/// markdown ones. Precedence between the groups is the collections'
/// business, not this module's - the scanner reports every run it finds
/// and lets them overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Construct {
    /// A reference to attached context, drawn as a chip.
    Attachment,
    /// A `/command` token at the head of its line.
    SlashToken,
    /// An `@path` mention at a word boundary.
    Mention,
    /// `*text*` or `_text_`.
    Emphasis,
    /// `**text**` or `__text__`.
    Strong,
    /// `` `text` ``.
    InlineCode,
    /// A fenced block, fences included.
    CodeFence,
    /// An ATX heading line, `#` markers included.
    Heading,
    /// The `-`, `*`, `+` or `1.` at the head of a list item - the marker
    /// only, not the item's text.
    ListMarker,
    /// A `>` quoted line.
    BlockQuote,
    /// `[text](target)`.
    Link,
}

/// One classified run of the buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    /// Byte range into the buffer. Always on character boundaries.
    pub(crate) range:     Range<usize>,
    pub(crate) construct: Construct,
}

impl Span {
    pub(crate) fn new(range: Range<usize>, construct: Construct) -> Self {
        Self { range, construct }
    }
}

/// One edit to the buffer, as the composer reports it.
///
/// `replaced` is a range in the text *before* the edit; `inserted` is how
/// many bytes took its place. A pure insertion is an empty `replaced`, a
/// pure deletion is `inserted: 0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Edit {
    pub(crate) replaced: Range<usize>,
    pub(crate) inserted: usize,
}

impl Edit {
    /// The range the inserted text occupies in the text *after* the edit.
    pub(crate) fn inserted_range(&self) -> Range<usize> {
        self.replaced.start..self.replaced.start + self.inserted
    }

    /// How far every byte after the edit moved.
    fn shift(&self) -> isize {
        self.inserted as isize - self.replaced.len() as isize
    }
}

/// The token spans on the line containing `offset`.
///
/// The lookup asks this to decide which trigger the caret is in, so that
/// what opens the popup and what gets the token treatment are one answer
/// rather than two rules that can drift apart.
pub(crate) fn tokens_on_line(text: &str, offset: usize) -> Vec<Span> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Vec::new();
    }
    let mut spans = tokens::scan(text, offset..offset);
    sort(&mut spans);
    spans
}

/// Every construct in `text`, given the attachment references currently
/// held in the pending-context table.
///
/// Spans come back sorted by start offset, then by construct, so two scans
/// of equal buffers are equal - which is what makes the incremental path
/// checkable against this one.
pub(crate) fn scan(text: &str, attachments: &[&str]) -> Vec<Span> {
    let mut spans = markdown::scan(text, 0..text.len());
    spans.extend(tokens::scan(text, 0..text.len()));
    spans.extend(attachment_spans(text, attachments));
    sort(&mut spans);
    spans
}

/// The constructs in `text` after `edit`, reusing `previous` for the parts
/// the edit cannot have changed.
///
/// Equivalent to [`scan`] on the new text, and only worth having because
/// it does not read all of it.
pub(crate) fn rescan(text: &str, attachments: &[&str], previous: &[Span], edit: &Edit)
                     -> Vec<Span> {
    let dirty = dirty::dirty_range(text, edit);

    // Attachment references are found by searching the buffer for each
    // one, which is already proportional to the table rather than to the
    // buffer's constructs, and a chip can move without its line being
    // edited. Redoing them whole is cheaper than reasoning about it.
    let mut spans: Vec<Span> = previous.iter()
                                       .filter(|span| span.construct != Construct::Attachment)
                                       .filter_map(|span| shift(span, edit))
                                       .filter(|span| !overlaps(&span.range, &dirty))
                                       .collect();

    spans.extend(markdown::scan(text, dirty.clone()));
    spans.extend(tokens::scan(text, dirty));
    spans.extend(attachment_spans(text, attachments));
    sort(&mut spans);
    spans
}

/// Where each attachment reference sits in the buffer, for every
/// occurrence: the same file attached twice is two chips.
fn attachment_spans(text: &str, attachments: &[&str]) -> Vec<Span> {
    let mut spans = Vec::new();
    for reference in attachments.iter().filter(|reference| !reference.is_empty()) {
        let mut from = 0;
        while let Some(found) = text[from..].find(*reference) {
            let start = from + found;
            spans.push(Span::new(start..start + reference.len(), Construct::Attachment));
            from = start + reference.len();
        }
    }
    spans
}

/// `span`, moved to where it sits after `edit` - or dropped, when the edit
/// landed inside it and it has to be found again rather than moved.
fn shift(span: &Span, edit: &Edit) -> Option<Span> {
    if span.range.end <= edit.replaced.start {
        return Some(span.clone());
    }
    if span.range.start >= edit.replaced.end {
        let shift = edit.shift();
        let start = span.range.start.checked_add_signed(shift)?;
        let end = span.range.end.checked_add_signed(shift)?;
        return Some(Span::new(start..end, span.construct));
    }
    None
}

/// Whether two ranges share any byte, counting a zero-length range that
/// sits inside the other as a touch.
fn overlaps(span: &Range<usize>, dirty: &Range<usize>) -> bool {
    span.start < dirty.end && dirty.start < span.end
    || span.is_empty() && dirty.contains(&span.start)
}

/// One order, so equal buffers give equal output whichever path produced
/// it.
fn sort(spans: &mut Vec<Span>) {
    spans.sort_by(|a, b| {
             a.range
              .start
              .cmp(&b.range.start)
              .then(a.range.end.cmp(&b.range.end))
              .then(a.construct.cmp(&b.construct))
         });
    spans.dedup();
}
