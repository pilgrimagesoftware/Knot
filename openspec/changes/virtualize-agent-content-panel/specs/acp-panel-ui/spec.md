# Spec Delta

## ADDED Requirements

### Requirement: Conversation rendering is virtualized

The conversation in the panel SHALL render through a virtualized list
scroller that lays out, measures and paints only the rows near the viewport
(plus a small overdraw margin), never the whole history. The per-frame work
and the retained layout state SHALL therefore be bounded by how much of the
conversation fits on screen, not by the conversation's total length, so a
long session does not get measurably more expensive the longer it runs.

Messages that have scrolled out of the viewport SHALL NOT be destroyed; they
are skipped for that frame's layout and painting only, so their full content
is still reachable by scrolling. Materializing a row later SHALL yield the
same content and state it would have had if the whole conversation had been
built eagerly.

#### Scenario: A long conversation stays cheap to render

- **WHEN** a conversation contains far more messages than can fit on screen at
  once
- **THEN** the panel lays out and paints only the messages near the viewport,
  and remains fluent as the user scrolls, regardless of how long the
  conversation is

#### Scenario: A previously skipped message still has its content

- **WHEN** the user scrolls to a message that is not currently materialized
- **THEN** that message renders with the full content and state it would have
  had if the whole conversation had been built eagerly

### Requirement: Streaming keeps the visible message anchored

As a message streams and grows, the row it belongs to SHALL be re-measured and
updated in place, without re-laying-out or tearing down the rest of the
conversation, so the content the user is currently reading does not jump. When
the user is following the tail, the panel SHALL stay at the end of the
conversation so newly streamed output remains visible as it arrives.

#### Scenario: Streaming output while reading earlier history

- **WHEN** the user has scrolled away from the tail and a visible message
  gains more streamed text
- **THEN** the growing row is re-measured in place and what the user is
  reading stays where it is

#### Scenario: Streaming output while following the tail

- **WHEN** the panel is following the tail and a new message begins to stream
- **THEN** the panel stays at the end of the conversation and the new content
  appears as it streams

### Requirement: Per-message and jump controls reach any message

The panel's scroll controls (scroll to the originating user message, scroll to
top, and jump to latest) SHALL operate on the virtualized list and SHALL reach
any message even if it is not currently materialized.

#### Scenario: Scroll to an unmaterialized user message

- **WHEN** the user activates "scroll to user input" for a response whose user
  message lies far above the current viewport
- **THEN** the panel scrolls that user message into view

#### Scenario: Jump to latest resumes tail following

- **WHEN** the user activates the jump-to-latest control after scrolling the
  history away from the tail
- **THEN** the panel scrolls to the newest message and resumes following
  streamed output

#### Scenario: Scroll to top from anywhere

- **WHEN** the user activates the scroll-to-top control
- **THEN** the panel shows the earliest message regardless of how far it is
  from the viewport

## MODIFIED Requirements

### Requirement: Track response toggle

The response action bar SHALL include a track toggle. While enabled for a
given response, the panel's virtualized list SHALL keep following the streamed
output for that response, auto-scrolling so the newest output stays visible.
Manually scrolling the history away from the tail SHALL disable following.
Disabling the toggle SHALL stop following even while the list is at the tail,
so following is only ever resumed by enabling the toggle or jumping to latest.

#### Scenario: Streaming update while the tail is being followed

- **WHEN** the user enables the track toggle on an in-progress response and
  the panel is following the tail
- **THEN** the list stays at the end and the newest streamed output is
  visible as it arrives

#### Scenario: Manual scroll disables following

- **WHEN** the user enables the track toggle on an in-progress response and
  then scrolls the history away from the tail
- **THEN** the list stops auto-scrolling, so the content the user scrolled to
  stays put as further output streams

#### Scenario: Turning the toggle off stops following even at the tail

- **WHEN** the user disables the track toggle on an in-progress response
- **THEN** the list stops auto-scrolling and stays where it is as further
  output streams, even if it is at the tail, rather than snapping back to the
  end
