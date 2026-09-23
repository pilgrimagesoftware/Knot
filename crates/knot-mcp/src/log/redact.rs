//! What a tool call looks like in the log file, as opposed to on standard
//! error.
//!
//! Standard error prints a call's arguments in full and always has. The file
//! does not: tool-call arguments carry prompt text, message bodies and file
//! paths, and a file in the user's Logs directory outlives the stream by
//! months. So the file records the *shape* of a call - which tool, which
//! argument names, how much payload - and never its content.
//!
//! Building both messages at the one call site is deliberate. A filter
//! applied somewhere downstream would be something a later caller could
//! bypass by logging through a different path; here the difference between
//! the two destinations is visible in the code that creates it.

use serde_json::Value;

/// The log file's description of a `tools/call`: the tool's name, the names
/// of its top-level arguments, and the size of the argument payload.
///
/// Argument *values* never appear. A non-object payload (a bare array or
/// string, which the protocol does not use but a client may still send) has
/// no keys to name, and is reported by size alone.
pub fn describe_call(name: &str, arguments: &Value) -> String {
    let bytes = arguments.to_string().len();
    match arguments.as_object() {
        Some(fields) => {
            let keys: Vec<&str> = fields.keys().map(String::as_str).collect();
            format!("tools/call {name} keys=[{}] bytes={bytes}", keys.join(", "))
        }
        None => format!("tools/call {name} keys=<not an object> bytes={bytes}"),
    }
}

#[cfg(test)]
mod tests;
