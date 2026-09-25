# Spec Delta

## MODIFIED Requirements

### Requirement: Each log entry is one self-describing line

Every entry in the log file SHALL occupy exactly one line holding one JSON object, so the file
is in JSON Lines format. The object SHALL carry these fields, in this order:

- `time`: a UTC timestamp in RFC 3339 form, including the date and a sub-second component;
- `pid`: the ID of the process that wrote the entry, as a number;
- `level`: the severity level;
- `subject`: a short word naming what the entry is about;
- `message`: the entry's message.

The process ID is required because every Knot instance that shares a home directory appends to
the same log file; without it a line cannot be attributed to the installed app rather than to a
development or test instance running beside it.

An entry SHALL NOT contain a raw newline: control characters inside a value SHALL be escaped
as JSON requires, so the line stays a line. One entry is therefore always one line, and the file
can be read line-wise by any JSON Lines reader.

The log file SHALL be named with a `.jsonl` extension.

#### Scenario: Entry shape

- **WHEN** any event is logged
- **THEN** its line parses as a JSON object carrying `time`, `pid`, `level`, `subject` and
  `message`, in that order

#### Scenario: The process ID is the writer's

- **WHEN** an entry is written to the log file
- **THEN** its `pid` is that of the process that logged it

#### Scenario: An embedded newline does not break a line

- **WHEN** a value that would be logged contains a newline
- **THEN** it is escaped and the entry remains a single line

#### Scenario: A message survives unchanged

- **WHEN** a message containing backslashes, quotes or control characters is logged
- **THEN** parsing its line yields the original message exactly
