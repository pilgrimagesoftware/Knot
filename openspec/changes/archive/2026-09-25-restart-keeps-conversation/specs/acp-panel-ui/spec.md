# Spec Delta

## ADDED Requirements

### Requirement: A loaded conversation shows its history
When a Panel-mode agent's session is loaded rather than created - a restart
that keeps the conversation, or a layout restore - the panel SHALL show the
conversation the agent replays, with each of the user's prompts as its own
user message and each reply as its own assistant message, in the order
replayed. Consecutive chunks of one prompt SHALL join into one message.

The loaded conversation SHALL open scrolled to its newest message, not its
first: a reader coming back to it wants where it left off.

A replayed prompt SHALL NOT start a turn: it was answered long ago, and the
composer stays usable. A user message chunk that arrives while a turn is in
flight SHALL be ignored, because the panel has already recorded the prompt it
sent, and showing an echo would duplicate it.

#### Scenario: Restarting keeps the visible conversation
- **WHEN** an agent whose conversation holds two prompts and two replies is
  restarted keeping its conversation
- **THEN** the panel shows the two prompts and the two replies, alternating,
  rather than the replies run together with no prompts

#### Scenario: A long loaded conversation opens at its end
- **WHEN** an agent with a conversation longer than its panel is restarted
  keeping it
- **THEN** the panel shows the newest message, with the earlier ones above it

#### Scenario: An echoed prompt is not doubled
- **WHEN** the user sends a prompt and the agent echoes it as a user message
  chunk during the turn
- **THEN** the prompt appears once
