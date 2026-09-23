# Tasks

## 1. Unblock the summary: sample while shown

- [x] 1.1 Change `agent_processes::observed_roots` to gate on the shown agent rather than on an expanded section, dropping the `sections` argument it no longer reads; verify the signature change forces every caller to be revisited rather than silently compiling
- [x] 1.2 Rewrite the gate's tests for the new rule — the shown agent is observed, a collapsed section is still observed, a stopped agent is not, a takeover view observes nothing; verify the collapsed case fails against the old gate
- [x] 1.3 Assert the cost bound holds: at most one agent observed per pass, so it stays one `ps -A` per interval per window
- [x] 1.4 Stop `toggle_process_section` clearing the snapshot on collapse; verify the collapsed header keeps summarizing after a collapse
- [x] 1.5 Create a section entry in `drain_published_processes` for any agent a pass reported, since `toggle` was previously the only thing that made one and the agents this fix is for have never been toggled

## 2. The summary itself

- [x] 2.1 Add `render/processes_summary.rs` with `process_name` — leading word of the command line, less its directories; verify `/opt/homebrew/bin/node --inspect server.js` names `node`, and an empty command names nothing rather than panicking
- [x] 2.2 Add `summary_text(is_running, expanded, processes)`: not running or empty reads as none, no sample reads as unknown, expanded counts, collapsed names; verify all five branches resolve from the catalog with no placeholder left behind
- [x] 2.3 Deduplicate names and cap the list, with a remainder counting the processes the shown names do not cover; verify eight `node` processes read as `node` once, and that the remainder counts processes rather than names
- [x] 2.4 Add the catalog entries — `count_none`, `count_one`/`count_many`, `summary_separator`, `summary_more` — and drop `processes.count`, which counted background descendants for a header that no longer does; verify tests assert the keys resolve, never the English copy

## 3. Wire it to the header

- [x] 3.1 Resolve `is_running` and the summary in `processes_section` and pass the rendered string to `processes_header`; verify the header no longer takes a count
- [x] 3.2 Truncate the summary with `min_w_0` + ellipsis so a collapsed header naming several processes cannot widen the pane
- [x] 3.3 Remove `ProcessSection::background_count`, superseded by the summary, and update the tests that asserted through it; verify no crate-wide `allow` was added to hide it as dead code instead

## 4. Gate

- [x] 4.1 Run `make` end to end — `fmt-check`, `size-check`, `lint`, `test`, `build`; verify no file crossed 700 lines
- [ ] 4.2 Verify by hand in a debug build: a collapsed section on a busy agent names its processes and updates as they come and go; expanding switches to a count; stopping the agent reads as none; a takeover view stops the sampling
