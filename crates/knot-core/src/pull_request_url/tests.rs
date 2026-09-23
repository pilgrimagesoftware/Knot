//! Unit tests for [`super`].

use super::*;

const HOSTS: &[&str] = &[PULL_REQUEST_HOST_GITHUB];

fn one(text: &str) -> PullRequestUrl {
    let found = scan_pull_request_urls(text);
    assert_eq!(found.len(),
               1,
               "expected exactly one URL in {text:?}, got {found:?}");
    found.into_iter().next().expect("length checked")
}

#[test]
fn a_bare_url_parses_into_its_parts() {
    let url = one("opened https://github.com/acme/widget/pull/42");
    assert_eq!(url.host, "github.com");
    assert_eq!(url.owner, "acme");
    assert_eq!(url.repo, "widget");
    assert_eq!(url.number, 42);
}

#[test]
fn a_files_suffix_is_not_part_of_the_pull_request() {
    assert_eq!(one("https://github.com/acme/widget/pull/42/files").number,
               42);
}

#[test]
fn a_comment_fragment_is_not_part_of_the_pull_request() {
    assert_eq!(one("https://github.com/acme/widget/pull/42#issuecomment-12345").number,
               42);
}

#[test]
fn a_query_string_is_not_part_of_the_pull_request() {
    assert_eq!(one("https://github.com/acme/widget/pull/42?w=1").number, 42);
}

/// The spec's line: Knot records pull requests, not links.
#[test]
fn an_issues_url_is_not_a_pull_request() {
    assert!(scan_pull_request_urls("https://github.com/acme/widget/issues/42").is_empty());
}

#[test]
fn a_repo_root_url_is_not_a_pull_request() {
    assert!(scan_pull_request_urls("https://github.com/acme/widget").is_empty());
}

#[test]
fn a_non_github_host_is_not_recorded() {
    assert!(scan_pull_request_urls("https://gitlab.com/acme/widget/pull/42").is_empty());
    assert!(scan_pull_request_urls("https://notgithub.com/acme/widget/pull/42").is_empty());
}

/// An Enterprise host is only recognized when something that knows the
/// installation says so - never guessed at from the URL's shape.
#[test]
fn an_enterprise_host_is_recorded_only_when_listed() {
    let text = "https://git.acme.example/acme/widget/pull/7";
    assert!(scan_pull_request_urls(text).is_empty());

    let found = scan_pull_request_urls_on_hosts(text, &["git.acme.example"]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].host, "git.acme.example");
    assert_eq!(found[0].number, 7);
}

#[test]
fn several_urls_in_one_string_are_all_found() {
    let found = scan_pull_request_urls("first https://github.com/acme/widget/pull/1 then \
                                        https://github.com/acme/widget/pull/2 and \
                                        https://github.com/other/thing/pull/3.");
    let numbers = found.iter().map(|url| url.number).collect::<Vec<_>>();
    assert_eq!(numbers, vec![1, 2, 3]);
    assert_eq!(found[2].owner, "other");
    assert_eq!(found[2].repo, "thing");
}

#[test]
fn a_zero_or_missing_number_is_not_a_pull_request() {
    assert!(scan_pull_request_urls("https://github.com/acme/widget/pull/0").is_empty());
    assert!(scan_pull_request_urls("https://github.com/acme/widget/pull/").is_empty());
    assert!(scan_pull_request_urls("https://github.com/acme/widget/pull/abc").is_empty());
}

/// Trailing punctuation belongs to the prose, not to the number.
#[test]
fn a_url_at_the_end_of_a_sentence_parses() {
    assert_eq!(one("See https://github.com/acme/widget/pull/42.").number,
               42);
    assert_eq!(one("(https://github.com/acme/widget/pull/42)").number, 42);
}

#[test]
fn a_url_embedded_in_a_longer_token_is_not_matched() {
    assert!(scan_pull_request_urls("xhttps://github.com/acme/widget/pull/42").is_empty());
}

// --- Canonical form --------------------------------------------------------

/// Two spellings of one pull request have to compare equal, or the same
/// pull request is recorded twice.
#[test]
fn the_files_and_bare_forms_produce_the_same_canonical_string() {
    let bare = one("https://github.com/acme/widget/pull/42");
    let files = one("https://github.com/acme/widget/pull/42/files");

    assert_eq!(bare.to_string(), "https://github.com/acme/widget/pull/42");
    assert_eq!(bare.to_string(), files.to_string());
    assert_eq!(bare, files);
}

#[test]
fn different_pull_requests_do_not_collide() {
    let first = one("https://github.com/acme/widget/pull/42");
    let second = one("https://github.com/acme/widget/pull/43");
    let other_repo = one("https://github.com/acme/gadget/pull/42");

    assert_ne!(first, second);
    assert_ne!(first, other_repo);
}

// --- The streaming scanner -------------------------------------------------

#[test]
fn a_url_fed_one_byte_at_a_time_is_still_found() {
    let text = "opened https://github.com/acme/widget/pull/42 just now";
    let mut scanner = PullRequestUrlScanner::new();
    let mut found = Vec::new();
    for byte in text.as_bytes() {
        found.extend(scanner.feed(&[*byte], HOSTS));
    }
    found.extend(scanner.flush(HOSTS));

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].number, 42);
}

/// The reason a match touching the end of a chunk is held back: `/pull/4`
/// followed by `2` is pull request 42, and reporting 4 would record a pull
/// request that does not exist.
#[test]
fn a_number_split_across_chunks_is_one_number() {
    let mut scanner = PullRequestUrlScanner::new();
    let mut found = scanner.feed(b"https://github.com/acme/widget/pull/4", HOSTS);
    assert!(found.is_empty(), "{found:?}");
    found.extend(scanner.feed(b"2 done", HOSTS));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].number, 42);
}

#[test]
fn a_url_ending_exactly_at_the_end_of_the_stream_is_flushed() {
    let mut scanner = PullRequestUrlScanner::new();
    assert!(scanner.feed(b"https://github.com/acme/widget/pull/42", HOSTS)
                   .is_empty());

    let found = scanner.flush(HOSTS);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].number, 42);
}

#[test]
fn a_url_is_reported_once_even_though_the_tail_is_carried() {
    let mut scanner = PullRequestUrlScanner::new();
    let mut found = scanner.feed(b"https://github.com/acme/widget/pull/42 ", HOSTS);
    found.extend(scanner.feed(b"and nothing more", HOSTS));
    found.extend(scanner.flush(HOSTS));

    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn two_urls_split_across_three_chunks_are_both_found() {
    let mut scanner = PullRequestUrlScanner::new();
    let mut found = scanner.feed(b"https://github.com/acme/widget/pull/1 https://git", HOSTS);
    found.extend(scanner.feed(b"hub.com/acme/widget/pu", HOSTS));
    found.extend(scanner.feed(b"ll/2 end", HOSTS));
    found.extend(scanner.flush(HOSTS));

    let numbers = found.iter().map(|url| url.number).collect::<Vec<_>>();
    assert_eq!(numbers, vec![1, 2]);
}

#[test]
fn a_megabyte_of_unrelated_output_yields_nothing() {
    let mut scanner = PullRequestUrlScanner::new();
    let chunk = b"the quick brown fox jumps over the lazy dog 0123456789 https://example.com/x\n";
    let mut found = Vec::new();
    let mut written = 0;
    while written < 1024 * 1024 {
        found.extend(scanner.feed(chunk, HOSTS));
        written += chunk.len();
    }
    found.extend(scanner.flush(HOSTS));

    assert!(found.is_empty(), "{found:?}");
}

/// A PTY carries whatever the program wrote. Invalid UTF-8 around a URL is
/// not a reason to miss it, or to panic.
#[test]
fn invalid_utf8_around_a_url_does_not_stop_the_scan() {
    let mut stream = vec![0xFF, 0xFE, 0x80];
    stream.extend_from_slice(b"https://github.com/acme/widget/pull/42 ");
    stream.extend_from_slice(&[0xC3, 0x28]);

    let mut scanner = PullRequestUrlScanner::new();
    let mut found = scanner.feed(&stream, HOSTS);
    found.extend(scanner.flush(HOSTS));

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].number, 42);
}

/// The carry buffer is what makes a split URL findable; it must not grow
/// with the stream.
#[test]
fn the_carry_buffer_stays_bounded() {
    let mut scanner = PullRequestUrlScanner::new();
    for _ in 0..1000 {
        scanner.feed(&[b'x'; 1024], HOSTS);
    }
    assert!(scanner.carry.len() <= MAX_PULL_REQUEST_URL_LEN,
            "carried {} bytes",
            scanner.carry.len());
}

/// The scan runs on the thread that parses PTY bytes into cells, so its cost
/// has to stay linear in the stream rather than merely bounded.
///
/// Asserted as work rather than as elapsed time (#369). knot-terminal used to
/// make this claim by timing a feed with the scan against one without it and
/// comparing, which measured whatever else the machine was doing and failed
/// in the suite while passing alone. Bytes handed to the scan are the thing
/// the claim is actually about, and counting them cannot be descheduled.
///
/// [`the_carry_buffer_stays_bounded`] pins the invariant this rests on; this
/// pins the consequence. They fail together if the carry grows without
/// bound, and this one alone catches a scan that re-reads its buffer more
/// than once per chunk.
#[test]
fn the_streaming_scan_does_work_linear_in_the_stream() {
    const CHUNKS: u64 = 2_000;
    let chunk = b"the quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnop\r\n";

    BYTES_SCANNED.with(|scanned| scanned.set(0));
    let mut scanner = PullRequestUrlScanner::new();
    for _ in 0..CHUNKS {
        scanner.feed(chunk, HOSTS);
    }
    let scanned = BYTES_SCANNED.with(core::cell::Cell::get);

    // One pass over each chunk, plus whatever was carried into it. The carry
    // is capped, so the ceiling is linear; a quadratic scan blows past it
    // long before the last chunk.
    let ceiling = CHUNKS * (chunk.len() + MAX_PULL_REQUEST_URL_LEN) as u64;
    assert!(scanned <= ceiling,
            "scanned {scanned} bytes over {CHUNKS} chunks of {} bytes; linear ceiling is {ceiling}",
            chunk.len());

    // And it did look at the stream - a scan that examined nothing would
    // satisfy the ceiling while finding nothing either.
    assert!(scanned >= CHUNKS * chunk.len() as u64,
            "scanned only {scanned} bytes; the stream alone is {}",
            CHUNKS * chunk.len() as u64);
}
