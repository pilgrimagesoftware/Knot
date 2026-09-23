//! Parses the output of `ps -Ao pid=,ppid=,pgid=,tpgid=,etime=,command=`.
//!
//! One line is one process: `ps` never emits an embedded newline in the
//! command column. The first five whitespace-delimited fields are numeric and
//! the rest of the line is the command verbatim, spaces and all.
//!
//! A malformed line is skipped rather than failing the sample. One odd row --
//! a kernel thread with an unexpected column, a process that exited while `ps`
//! was writing -- must not blank the whole section.

use std::time::Duration;

use crate::consts::PS_LEADING_FIELDS;

/// One row of the process table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRecord {
    pub pid:     u32,
    pub ppid:    u32,
    /// The process's own process group.
    pub pgid:    u32,
    /// The foreground process group of this process's controlling terminal, or
    /// `0`/`-1` when it has none.
    pub tpgid:   i32,
    /// Time since the process started. Resolution is one second, which is all
    /// `etime` carries.
    pub elapsed: Duration,
    pub command: String,
}

impl ProcessRecord {
    /// Whether this process sits in its terminal's foreground process group.
    ///
    /// A process with no controlling terminal reports a `tpgid` of `0` or
    /// `-1`, neither of which is a valid process group, so it answers `false`
    /// without being special-cased.
    pub fn is_foreground(&self) -> bool {
        self.tpgid > 0 && self.tpgid as u32 == self.pgid
    }
}

/// Parses whole `ps` output, skipping any line that does not parse.
pub fn parse_table(text: &str) -> Vec<ProcessRecord> {
    text.lines().filter_map(parse_record).collect()
}

/// Parses one `ps` line, or `None` if it is not a well-formed record.
pub fn parse_record(line: &str) -> Option<ProcessRecord> {
    let line = line.trim_start();
    let mut rest = line;
    let mut fields = [0_i64; PS_LEADING_FIELDS - 1];

    // `ps` pads numeric columns with leading spaces, so split on runs of
    // whitespace rather than a single separator.
    for field in fields.iter_mut() {
        let (value, tail) = next_field(rest)?;
        *field = value.parse().ok()?;
        rest = tail;
    }

    let (etime, tail) = next_field(rest)?;
    let elapsed = parse_etime(etime)?;
    // Only the whitespace `ps` used to separate the columns is dropped; what is
    // left is the command line as the kernel reports it, spaces and all.
    let command = tail.trim();

    if command.is_empty() {
        return None;
    }

    Some(ProcessRecord { pid: u32::try_from(fields[0]).ok()?,
                         ppid: u32::try_from(fields[1]).ok()?,
                         pgid: u32::try_from(fields[2]).ok()?,
                         tpgid: i32::try_from(fields[3]).ok()?,
                         elapsed,
                         command: command.to_owned() })
}

/// Splits the leading whitespace-delimited field from `rest`, returning it and
/// what follows the whitespace that ended it.
fn next_field(rest: &str) -> Option<(&str, &str)> {
    let rest = rest.trim_start();
    let end = rest.find(char::is_whitespace)?;

    Some((&rest[..end], &rest[end..]))
}

/// Parses `ps`'s `etime` column: `[[DD-]HH:]MM:SS`.
pub fn parse_etime(text: &str) -> Option<Duration> {
    let (days, clock) = match text.split_once('-') {
        Some((days, clock)) => (days.parse::<u64>().ok()?, clock),
        None => (0, text),
    };

    let mut parts = clock.split(':');
    let first: u64 = parts.next()?.parse().ok()?;
    let second: u64 = parts.next()?.parse().ok()?;
    let third = parts.next().map(str::parse::<u64>).transpose().ok()?;

    if parts.next().is_some() {
        return None;
    }

    let (hours, minutes, seconds) = match third {
        Some(seconds) => (first, second, seconds),
        None => (0, first, second),
    };

    if minutes > 59 || seconds > 59 {
        return None;
    }

    Some(Duration::from_secs(((days * 24 + hours) * 60 + minutes) * 60 + seconds))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ProcessRecord, parse_etime, parse_record, parse_table};

    /// Captured from `ps -Ao pid=,ppid=,pgid=,tpgid=,etime=,command=` on
    /// macOS: padded numeric columns, a daemon with no controlling terminal, a
    /// command containing spaces, and a foreground shell.
    const FIXTURE: &str = "\
    1     0     1    0 04-05:24:46 /sbin/launchd
  336     1   336    0 04-05:24:03 /usr/libexec/logd
86481 86477 86481 86481    01:02:03 -zsh
86667 86481 86481 86481       48:08 docker run --rm -i grafana/mcp-grafana -t stdio
89630 86481 86481 86481       01:33 caffeinate -i -t 300
";

    #[test]
    fn parses_every_well_formed_line() {
        let table = parse_table(FIXTURE);

        assert_eq!(table.len(), 5);
        assert_eq!(table[0].pid, 1);
        assert_eq!(table[0].ppid, 0);
    }

    #[test]
    fn keeps_a_command_containing_spaces_verbatim() {
        let table = parse_table(FIXTURE);
        let docker = table.iter().find(|record| record.pid == 86667).unwrap();

        assert_eq!(docker.command,
                   "docker run --rm -i grafana/mcp-grafana -t stdio");
    }

    #[test]
    fn tolerates_leading_whitespace_padding() {
        let record =
            parse_record("      42       1      42       0       01:00 /usr/bin/true").unwrap();

        assert_eq!(record.pid, 42);
        assert_eq!(record.ppid, 1);
        assert_eq!(record.elapsed, Duration::from_secs(60));
    }

    #[test]
    fn skips_a_malformed_line_without_failing_the_sample() {
        let text = format!("{FIXTURE}not a process record at all\n  9 8 not-a-number 0 01:00 x\n");
        let table = parse_table(&text);

        assert_eq!(table.len(),
                   5,
                   "the malformed lines were skipped, the rest survived");
    }

    #[test]
    fn a_record_with_no_command_is_skipped() {
        assert!(parse_record("42 1 42 0 01:00").is_none());
        assert!(parse_record("42 1 42 0 01:00   ").is_none());
    }

    #[test]
    fn parses_both_etime_shapes() {
        assert_eq!(parse_etime("48:08"), Some(Duration::from_secs(48 * 60 + 8)));
        assert_eq!(parse_etime("01:02:03"), Some(Duration::from_secs(3723)));
        assert_eq!(parse_etime("04-05:24:46"),
                   Some(Duration::from_secs(4 * 86_400 + 5 * 3600 + 24 * 60 + 46)));
    }

    #[test]
    fn rejects_an_unparseable_etime() {
        assert_eq!(parse_etime(""), None);
        assert_eq!(parse_etime("48"), None);
        assert_eq!(parse_etime("1:2:3:4"), None);
        assert_eq!(parse_etime("01:99"), None);
        assert_eq!(parse_etime("x-01:02:03"), None);
    }

    #[test]
    fn foreground_is_own_group_matching_the_terminals() {
        let table = parse_table(FIXTURE);

        let shell = table.iter().find(|record| record.pid == 86481).unwrap();
        assert!(shell.is_foreground());

        let logd = table.iter().find(|record| record.pid == 336).unwrap();
        assert!(!logd.is_foreground(),
                "a daemon has no controlling terminal");
    }

    #[test]
    fn a_negative_foreground_group_is_not_foreground() {
        let record = ProcessRecord { pid:     7,
                                     ppid:    1,
                                     pgid:    7,
                                     tpgid:   -1,
                                     elapsed: Duration::from_secs(1),
                                     command: "sleep 1".to_owned(), };

        assert!(!record.is_foreground());
    }
}
