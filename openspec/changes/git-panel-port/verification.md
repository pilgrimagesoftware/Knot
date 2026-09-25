# Manual verification

Three tasks in `tasks.md` cannot be closed from a headless session: 3.5, 9.5
and 10.5. Everything else in this change is covered by automated tests.

This file exists so whoever has a display can close them in a few minutes
rather than re-deriving what to look at. Each step says what to do, what
should happen, and which spec scenario it discharges.

## Before you start

**Isolate the settings store.** Adding a workspace or agent writes to the
real store, and that store has already been clobbered once. Both documents
went together in a single event:

```
agents.json.clobbered-20260922-145115
workspaces.json.clobbered-20260922-145115
```

Same timestamp — the agent roster and the workspace list, not just one of
them. `with_store_root` exists as the test-only escape hatch because of it,
and the app itself does not use it.

A scratch `HOME` relocates the whole store. The chain, read out of crate
source rather than documentation:

```
StorePaths::platform()                   knot-core/src/settings/store/paths.rs:38
  -> ProjectDirs::from(ORG_QUALIFIER, ORG_NAME, APP_NAME)
  -> macOS base is $HOME/Library/Application Support/...
                                         directories-6.0.0/src/lib.rs:198
  -> dirs_sys::home_dir() reads $HOME first, falling back to getpwuid_r
     only when it is unset or empty
                                         dirs-sys-0.5.0/src/lib.rs:34
```

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

**Expect the MCP server to fail to bind** if another Knot is already running
— and note that `HOME` isolation does not help here, because the port is not
in the store. That is harmless for everything below, none of which touches
agent-to-agent messaging. It would not be harmless for anything that does.

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
printf 'deep\nchanged\n' > nested/deep.txt                    # a nested path
```

Confirm it produced all four states before launching — `1 MM` is the
staged-and-modified path that should draw two rows, `u UU` the conflict:

```bash
git -C /tmp/knot-verify-repo status --porcelain=v2 | grep -v '^# branch.oid'
```

Point a Claude (or any non-shell) agent at `/tmp/knot-verify-repo`; its
default panel mode is fine. Not a Shell agent: its header has no diff-stat
row, and that row is the only control that opens the git panel. The scratch
`HOME` has no Claude credentials, so the agent reports "authentication
required" — harmless, because the stats and the panel read the repository,
not the session.

To open the panel, select the agent (not the Dashboard), wait for the
header's top-right to change from "pending" to `+N −N`, and click it.

## Task 3.5 — no git command runs from a render

The rule this guards is the one `crates/knot/src/diff_stats.rs` exists for.
A render that shells out costs a subprocess per frame, and GPUI re-renders on
every keystroke.

**Do not try to watch for `git` processes with `pgrep`.** `git status` on a
small repository finishes in about 10ms, so a polling loop misses almost all
of them — you would see nothing and conclude it passed. Count the
invocations instead, by putting a counting shim earlier on `PATH`:

```bash
rm -rf /tmp/knot-git-count && mkdir -p /tmp/knot-git-count
cat > /tmp/knot-git-count/git <<'SH'
#!/bin/sh
printf '%s %s\n' "$(date +%H:%M:%S)" "$*" >> /tmp/knot-git-count/log
exec /usr/bin/git "$@"
SH
chmod +x /tmp/knot-git-count/git
```

The shim is honoured because `knot_core::exec_path::search_path` puts the
process's own `PATH` first, and the app is launched from a shell. Relaunch
with it in front:

```bash
HOME=/tmp/knot-verify-home PATH=/tmp/knot-git-count:$PATH cargo run -p knot
```

Then:

1. Open the panel and select a file, so a status and a diff are on screen.
2. **Validate the instrument before measuring.** Check the log has entries:
   ```bash
   wc -l < /tmp/knot-git-count/log
   ```
   It should be non-zero — opening the panel runs `status` and `diff`. **If
   it is empty, the shim is not being used and the rest of this check is
   meaningless**; a silent log would otherwise read as a perfect pass. Fix
   that before continuing.
3. Record the count, click **Commit**, and type continuously in the message
   field for ~15 seconds.
4. Record it again:
   ```bash
   wc -l < /tmp/knot-git-count/log
   ```

**Expected:** the two counts are equal, or differ by a small handful from a
background refresh. A count that climbs with your keystrokes — tens or
hundreds of new lines — is the defect.

`tail /tmp/knot-git-count/log` shows which commands, if it does climb.

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

## Task 9.5 — the dashboard card follows a commit

Step 19 above checks the agent *header*. The dashboard card is a second
surface reading the same cache, and it is the one that would keep stale
counts for a full `DIFF_STATS_MAX_AGE` (2s) if the panel failed to
invalidate.

25. With the panel open and something staged, switch the window to the
    Dashboard view and note the agent card's `+N -N` figures. Switch back,
    commit, and return to the Dashboard.

    **Expected:** the card shows the post-commit figures straight away. If it
    shows the pre-commit counts and corrects itself a second or two later,
    the panel is not invalidating `diff_stats` and is merely being rescued by
    the cache ageing out. → *Diff stats follow the panel*

## Clean up

```bash
rm -rf /tmp/knot-verify-home /tmp/knot-verify-repo
```

Then re-run the store checksum above and confirm the real store is unchanged.
