//! The blocking entry point: read the machine's process table once.
//!
//! Blocking, and on a busy machine not cheap -- wrap it in `spawn_blocking`.
//! One call serves every agent observed in a window; see [`crate::table`].

use crate::command::run_checked;
use crate::consts::{PS_ARGS, PS_PROGRAM};
use crate::error::Result;
use crate::record::parse_table;
use crate::table::ProcessTable;

/// Runs `ps`, parses its output, and returns the whole table.
pub fn sample() -> Result<ProcessTable> {
    let output = run_checked(PS_PROGRAM, PS_ARGS)?;

    Ok(ProcessTable::from_records(parse_table(&output)))
}
