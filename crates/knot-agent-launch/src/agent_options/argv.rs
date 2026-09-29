//! Options for an adapter that is the agent's own CLI in ACP mode, taken
//! on its command line after the adapter's fixed arguments.

use super::AdapterOptions;
use super::flags::split;
use crate::consts::GEMINI_OPTION_FLAGS;

/// Gemini's options: the flags in [`GEMINI_OPTION_FLAGS`], under their long
/// names, after `--acp --skip-trust`.
pub(super) fn gemini_options(words: Vec<String>) -> AdapterOptions {
    let (given, ignored) = split(words, &GEMINI_OPTION_FLAGS);
    AdapterOptions { args: given.iter().flat_map(|flag| flag.words()).collect(),
                     ignored,
                     ..AdapterOptions::default() }
}
