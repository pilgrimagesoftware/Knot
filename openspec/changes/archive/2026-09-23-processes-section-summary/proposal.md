# Proposal

## Why

The processes section reads "Counting…" forever — with processes running or
without, and it never updates.

The current spec asks for two things that cannot both hold:

- "Collapsed, the section SHALL show a label and the count of the agent's
  background descendants", and "A collapsed section's count SHALL come from a
  sample taken at the same interval".
- "Sampling for an agent SHALL run only while that agent's processes section
  is expanded and the agent is running".

The section starts collapsed. A collapsed section is not sampled. So the count
a collapsed section is required to show can never be computed, and the header
renders the unknown marker for the life of the window. Collapsing after an
expansion made it permanent: `toggle_process_section` called `clear()`, so the
one sample that had landed was discarded.

The count was also the wrong thing to show there. The question a collapsed
section answers is "is anything running, and what?" — and a bare number does
not say what. The number is worth having once the section is open, beside the
rows it counts.

## What Changes

- **The sampler's gate becomes "shown" rather than "expanded".** An agent's
  processes are sampled while its pane is on screen and it is running,
  whatever the section's disclosure state.
- **Collapsed, the header names what is running** — the distinct process
  names, deduplicated, up to three, with a remainder counting the processes
  those names do not cover.
- **Expanded, the header counts** the descendants listed below it.
- **Nothing running reads as "None"**, in both states, and for a stopped agent
  regardless of what its last sample held. Blank is not an option and neither
  is "Counting…".
- **"Counting…" narrows to when it is true**: a shown section whose first
  sample has not yet landed. That is now a moment rather than a permanent
  state.
- **Collapsing no longer discards the sample.** It was guarding against a
  stale list being the first thing drawn on reopening; sampling continues
  while collapsed, so there is no staleness left to guard.

### Cost

This is a real increase, and the reason it is bounded is worth stating.
Previously a window sampled only when the user had expanded a section —
usually never. Now it samples whenever a running agent's pane is shown.

The bound is unchanged in shape: at most one agent's pane is on screen per
window, so it remains one `ps -A` per `SAMPLE_INTERVAL` (3s) per window, off
the render path on a blocking task. It is not per agent and not per expanded
section. A window showing the dashboard or the pull requests list, or one
whose agent is not running, samples nothing at all.

### Non-goals

- A different sampling cadence for collapsed than for expanded. The collapsed
  header exists to show what is running *now*; a slower cadence there would
  make the summary the stalest thing on screen.
- Changing what a row shows, how rows are ordered, or the terminate and copy
  actions.
- Naming processes by anything cleverer than the leading word of the command
  line less its directories. `node` is what fits and what identifies.

## Capabilities

### Modified Capabilities

- `agent-processes` — what the section's header shows in each state, and when
  sampling runs.
