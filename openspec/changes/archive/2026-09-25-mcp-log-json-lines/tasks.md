## 1. Write JSON Lines

- [x] 1.1 Record the process ID on `Entry` where it is created, so it is the
      writer's even if the entry is rendered elsewhere
- [x] 1.2 Render an entry as one JSON object - `time`, `pid`, `level`,
      `subject`, `message`, in declaration order - and remove `escape`
- [x] 1.3 Rename `LOG_FILE_NAME` to `knot-mcp.jsonl`
- [x] 1.4 Make every test that asserted on the text format parse the line,
      and cover the field order, the process ID, newline and carriage-return
      escaping, and an exact round trip of an awkward message
- [x] 1.5 Run `make`
