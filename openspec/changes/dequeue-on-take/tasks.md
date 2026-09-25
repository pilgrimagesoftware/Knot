# Tasks

## 1. Queue

- [x] 1.1 Remove `QueuedPanelPrompt::in_flight`. Have `drain_panel_prompt`
  take the head out of the queue into a per-agent in-flight slot, and have
  `drain_prompt_results` clear the slot, putting a failed prompt back at the
  head marked failed. Verify with `prompt_queue` tests for take and for a
  failed delivery's return.

## 2. Gate

- [x] 2.1 Run `make`. It must pass.
- [ ] 2.2 Run the app, queue two prompts behind a running turn, and confirm
  each row disappears as its prompt is sent.
