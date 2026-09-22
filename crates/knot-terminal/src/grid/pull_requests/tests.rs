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
/// Asserted as a ratio rather than a wall-clock bound: an absolute threshold
/// would be a flaky test on a loaded CI machine, while the ratio between two
/// runs on the same machine is stable.
#[test]
fn scanning_does_not_measurably_slow_the_grid_feed() {
    const CHUNKS: usize = 2_000;
    let chunk = b"the quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnop\r\n";

    let mut scanning = grid();
    let started = Instant::now();
    for _ in 0..CHUNKS {
        scanning.feed(chunk);
    }
    let with_scan = started.elapsed();

    // The same work with the scan skipped, to price the VT parse alone.
    let mut parsing_only = grid();
    let started = Instant::now();
    for _ in 0..CHUNKS {
        parsing_only.feed_without_scan(chunk);
    }
    let without_scan = started.elapsed();

    println!("feed with scan: {with_scan:?}; parse alone: {without_scan:?}");
    assert!(with_scan.as_secs_f64() < without_scan.as_secs_f64() * 2.0 + 0.05,
            "scan cost {with_scan:?} against a {without_scan:?} parse");
}
