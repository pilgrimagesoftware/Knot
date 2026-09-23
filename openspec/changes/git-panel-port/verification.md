# Manual verification

Two tasks in `tasks.md` cannot be closed from a headless session: 3.5 and
10.5. Everything else in this change is covered by automated tests.

This file exists so whoever has a display can close them in a few minutes
rather than re-deriving what to look at. Each step says what to do, what
should happen, and which spec scenario it discharges.

## Before you start

**Isolate the settings store.** Adding a workspace or agent writes to the
real store, and the real store has already been clobbered once — there is an
`agents.json.clobbered-20260922-145115` beside it. `directories` resolves the
macOS base from `$HOME`, so a scratch `HOME` gives the app its own store:

```bash
mkdir -p /tmp/knot-verify-home
HOME=/tmp/knot-verify-home cargo run -p knot
```

Confirm it worked before going further — the real store must be untouched:

```bash
# before launching, and again after quitting: these must match
find "$HOME/Library/Application Support/com.Pilgrimage-Software.Knot" \
  -type f -print0 | sort -z | xargs -0 shasum
```

**Expect the MCP server to fail to bind** if another Knot is already running.
That is fine for everything below; none of it touches agent messaging.

**Make a repository with all four section types**, so one agent exercises the
whole panel:

```bash
R=/tmp/knot-verify-repo && rm -rf $R && mkdir -p $R && cd $R
git init -q -b main
git config user.email v@example.com && git config user.name V
printf 'one\n' > tracked.txt && printf 'base\n' > conflicted.txt
mkdir -p nested && printf 'deep\n' > nested/deep.txt
git add -A && git commit -qm init

# a conflict
git checkout -qb other && printf 'theirs\n' > conflicted.txt
git commit -qam theirs && git checkout -q main
printf 'ours\n' > conflicted.txt && git commit -qam ours
git merge other 2>/dev/null   # leaves conflicted.txt unmerged

printf 'one\nstaged\n' > tracked.txt && git add tracked.txt   # staged
printf 'one\nstaged\nunstaged\n' > tracked.txt                # and unstaged
printf 'new\n' > untracked.txt                                # untracked
```

Point an agent at `/tmp/knot-verify-repo`. A terminal-mode agent is enough —
the panel does not need a live ACP session.

## Task 3.5 — no git command runs from a render

The rule this guards is the one `crates/knot/src/diff_stats.rs` exists for.
A render that shells out costs a subprocess per frame, and GPUI re-renders on
every keystroke.

1. Open the panel (click the diff-stat row in the agent header) and select a
   file so a diff is on screen.
2. In another terminal, watch for git processes:
   ```bash
   while :; do pgrep -fl '(^|/)git ' | grep -v pgrep; sleep 0.2; done
   ```
3. Click **Commit** and type continuously in the message field for ~15
   seconds.

**Expected:** the watcher stays silent apart from at most one burst when the
panel first loads or refreshes. A steady stream of `git status` / `git diff`
while you type is the defect.

**Also worth noting:** the panel should not flicker or blank while typing.

Discharges: *Git work stays off the render path* — "Rendering issues no git
command" and "One request per outstanding refresh".

## Task 10.5 — the full review loop

Work down the panel. Each numbered item names the scenario it covers.

**Opening and layout**
1. The panel appears beside the agent's content, not over it, and the agent's
   terminal stays visible. → *Opening and dismissing the panel*
2. It opens 500pt wide. Drag its edge: it stops at roughly 350 and 800.
   Close and reopen — it is 500 again. → *Panel width and resizing*

**The tree**
3. Four sections in order: Staged Changes, Changes, Untracked, Conflicts,
   each with a count. → *Working tree sections*
4. `tracked.txt` appears **twice** — once under Staged, once under Changes —
   and the two rows show *different* change letters. → *A path staged and
   modified appears twice*, *File rows*
5. `nested/deep.txt` shows its directory on a second line; a root-level file
   shows none. → *File rows*
6. The branch line shows `main`. → *Branch header*

**Selection and diff**
7. Click the staged row of `tracked.txt`, then the unstaged row. They
   highlight independently and show **different** diffs. → *File selection*
8. The diff shows two line-number gutters; additions, deletions, context and
   `@@` hunk headers are each visually distinct; hunk headers have no line
   numbers. → *Diff rendering*
9. Make a very long line (`python3 -c "print('x'*400)" >> tracked.txt`) and
   select it: the line does **not** wrap, the pane scrolls sideways, and the
   gutters stay aligned. → *Long lines do not wrap*
10. Select `untracked.txt` before staging it: the pane says there are no
    changes rather than showing an empty box. → *Diff with no hunks*
11. Add a binary file (`head -c 200 /dev/urandom > blob.bin`), stage it, and
    select it: it reports a binary file. → *Binary file*

**Actions**
12. An untracked row offers only Stage. A staged row offers only Unstage. An
    unstaged tracked row offers Stage and Discard. → *Per-path staging
    actions*
13. Click Discard on an unstaged row: a confirmation naming that path
    appears, with a red confirm button. Cancel — the file is unchanged.
    → *Discard is confirmed* (this is an addition beyond the Swift app)
14. Stage and Unstage run with **no** confirmation. → *Staging is not
    confirmed*
15. Stage All also picks up untracked files. Untracked and Conflicts sections
    have no bulk button. → *Bulk staging actions*

**Commit**
16. With nothing staged, no Commit control. Stage something — it appears.
    → *Commit*
17. Open it; the confirm button is disabled while the message is empty or
    only spaces. → *Whitespace-only message cannot be committed*
18. Commit with a subject, a blank line, and a body. Check it survived:
    `git -C /tmp/knot-verify-repo log -1 --format=%B`. → *Message is trimmed
    but its body preserved*
19. The window closes and the panel refreshes. The agent header's diff-stat
    row updates **immediately**, not after a couple of seconds. → *Diff stats
    follow the panel*
20. Force a failure — add a rejecting hook and commit again:
    ```bash
    printf '#!/bin/sh\nexit 1\n' > /tmp/knot-verify-repo/.git/hooks/pre-commit
    chmod +x /tmp/knot-verify-repo/.git/hooks/pre-commit
    ```
    The window stays open, shows the error, and **still contains your typed
    message**. → *Failed commit keeps the message*

**Watching and errors**
21. With the panel open and idle, edit a file from a terminal. Within a
    second or two the list updates by itself. → *Refresh*
22. Stage a file from the panel: the list updates once. It should not
    refresh twice in quick succession. → *The panel's own writes do not
    trigger it*
23. Point an agent at a folder that is not a repository: the panel says so
    rather than showing a clean tree. → *Folder is not a repository*
24. Corrupt one: `echo garbage > /tmp/knot-verify-repo/.git/HEAD`. The panel
    shows an error, **not** "Working tree clean" — the Swift app showed the
    latter. → *Failed status is an error*

## Clean up

```bash
rm -rf /tmp/knot-verify-home /tmp/knot-verify-repo
```

Then re-run the store checksum above and confirm the real store is unchanged.
