//! Watching a terminal agent's PTY stream for pull request URLs.

use std::time::Instant;

use crate::grid::{Grid, GridSize};

const PULL_REQUEST: &str = "https://github.com/acme/widget/pull/42";

fn grid() -> Grid {
    Grid::new(GridSize { columns: 80,
                         rows:    24, })
}

#[test]
fn a_url_in_the_stream_is_noted() {
    let mut grid = grid();

    grid.feed(b"https://github.com/acme/widget/pull/42\r\n");

    assert_eq!(grid.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

/// The reason the scan is on the stream and not on the grid: a URL printed
/// into a narrow terminal is broken across rows by the renderer, and reading
/// it back off the screen would never reassemble it.
#[test]
fn a_url_wider_than_the_terminal_is_still_found() {
    let mut grid = Grid::new(GridSize { columns: 20,
                                        rows:    5, });

    grid.feed(format!("opened {PULL_REQUEST} ok\r\n").as_bytes());

    assert_eq!(grid.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

/// A PTY read can split a URL anywhere.
#[test]
fn a_url_split_across_reads_is_still_found() {
    let mut grid = grid();
    let bytes = format!("{PULL_REQUEST} done");

    for byte in bytes.as_bytes() {
        grid.feed(&[*byte]);
    }

    assert_eq!(grid.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

#[test]
fn the_same_url_printed_twice_is_buffered_once() {
    let mut grid = grid();

    grid.feed(format!("{PULL_REQUEST}\r\n").as_bytes());
    grid.feed(format!("{PULL_REQUEST}\r\n").as_bytes());

    assert_eq!(grid.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

#[test]
fn draining_clears_the_buffer() {
    let mut grid = grid();
    grid.feed(format!("{PULL_REQUEST}\r\n").as_bytes());

    assert_eq!(grid.take_pull_request_urls().len(), 1);
    assert!(grid.take_pull_request_urls().is_empty());
}

#[test]
fn ordinary_output_notes_nothing() {
    let mut grid = grid();

    grid.feed(b"$ cargo test\r\n   Compiling knot-terminal v1.13.0\r\n");
    grid.feed(b"https://github.com/acme/widget/issues/42\r\n");

    assert!(grid.take_pull_request_urls().is_empty());
}

/// The scan runs on the same thread that parses the bytes into cells, so it
/// has to be cheap relative to VT parsing rather than merely cheap. A
/// `yes`-style flood is the shape that would show it up.
///
/// Ignored, not deleted, and not part of the gate: this is a wall-clock
/// comparison and there is no way to make one deterministic. Run it by hand
/// when changing the scan - `cargo test -p knot-terminal -- --ignored
/// --nocapture` - and read the printed medians rather than trusting the
/// assertion alone.
///
/// The earlier version of this test compared two timings taken one after the
/// other and justified it as a ratio being stabler than an absolute bound
/// (#369). That reasoning does not hold: the two arms are measured at
/// different moments, so load arriving between them lands on one and not the
/// other, and the scanning arm ran first and so paid every cold cost -
/// allocation, page faults, cache warm-up. The bias was systematic and always
/// against the assertion. In the suite it read `with_scan` at 70.8ms against
/// a 9.2ms parse and failed; in isolation it read the scanning arm as *faster*
/// than the parse it contains.
///
/// What actually guards the feed is deterministic and lives in knot-core,
/// where the state is visible. `pull_request_url::tests` asserts both halves
/// without a clock: `the_carry_buffer_stays_bounded` pins the invariant, and
/// `the_streaming_scan_does_work_linear_in_the_stream` counts the bytes
/// handed to the scan and holds them under a linear ceiling. An unbounded
/// carry is what would make each scan re-read an ever-larger buffer and turn
/// the feed quadratic; both of those fail if it does, and the second alone
/// catches a scan that re-reads its buffer more than once per chunk. Counting
/// work is the honest form of this claim - it is what "does not measurably
/// slow the feed" means - and it cannot be descheduled.
///
/// Both arms are warmed and then interleaved across repetitions, and the
/// medians compared, so that a deliberate run is not measuring the order the
/// arms happen to run in.
#[test]
#[ignore = "wall-clock comparison; cannot be made deterministic (#369). \
            The bounded-carry invariant it stood in for is asserted in \
            knot-core::pull_request_url::tests."]
fn scanning_does_not_measurably_slow_the_grid_feed() {
    const CHUNKS: usize = 2_000;
    const REPEATS: usize = 9;
    let chunk = b"the quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnop\r\n";

    let feed_scanning = || {
        let mut grid = grid();
        let started = Instant::now();
        for _ in 0..CHUNKS {
            grid.feed(chunk);
        }
        started.elapsed()
    };
    // The same work with the scan skipped, to price the VT parse alone.
    let feed_parsing_only = || {
        let mut grid = grid();
        let started = Instant::now();
        for _ in 0..CHUNKS {
            grid.feed_without_scan(chunk);
        }
        started.elapsed()
    };

    // Discarded: the first run of either arm pays allocation and cache
    // warm-up that has nothing to do with the scan.
    feed_scanning();
    feed_parsing_only();

    let mut scanning = Vec::with_capacity(REPEATS);
    let mut parsing_only = Vec::with_capacity(REPEATS);
    for _ in 0..REPEATS {
        // Alternating, so a load spike lands on both arms rather than
        // whichever one happens to be running at the time.
        scanning.push(feed_scanning());
        parsing_only.push(feed_parsing_only());
    }
    scanning.sort_unstable();
    parsing_only.sort_unstable();
    let with_scan = scanning[REPEATS / 2];
    let without_scan = parsing_only[REPEATS / 2];

    println!("median of {REPEATS}: feed with scan {with_scan:?}; parse alone {without_scan:?}");
    assert!(with_scan.as_secs_f64() < without_scan.as_secs_f64() * 2.0 + 0.05,
            "scan cost {with_scan:?} against a {without_scan:?} parse");
}
