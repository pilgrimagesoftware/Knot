//! The single-versus-dual decision a section's header is drawn from.

use super::SectionChrome;
use super::section_chrome;

/// The toolbar's close-all already closes a lone section, so it has no close
/// of its own - nor a chevron, having nothing to give its height to. A
/// collapse left over from when both were open does not apply.
#[test]
fn a_lone_section_has_no_close_and_no_chevron() {
    for collapsed in [false, true] {
        assert_eq!(section_chrome(false, collapsed),
                   SectionChrome { collapsible: false,
                                   collapsed:   false,
                                   closable:    false, });
    }
}

/// With both open, each section's close takes it alone - something close-all
/// cannot do - so both levels stay.
#[test]
fn with_both_open_each_section_keeps_its_close_and_chevron() {
    assert_eq!(section_chrome(true, false),
               SectionChrome { collapsible: true,
                               collapsed:   false,
                               closable:    true, });
    assert_eq!(section_chrome(true, true),
               SectionChrome { collapsible: true,
                               collapsed:   true,
                               closable:    true, });
}
