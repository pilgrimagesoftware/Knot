# Tasks

## 1. Model

- [x] 1.1 `bug_report/screenshots.rs`: keep image files up to the cap, count the skipped ones, name them for the body; constants in `consts.rs`
- [x] 1.2 `Report.screenshots`; `issue_body`/`compose_body` name them under a heading only when there are any

## 2. Delivery

- [x] 2.1 `submit` takes a `reveal` seam beside `open_url`; after a filed report with screenshots, open the issue page and reveal the files; on the browser path, reveal them
- [x] 2.2 Screenshots-aware status (`Phase::BrowserReady`) and filed notification; a "by hand" variant where files cannot be revealed

## 3. Dialog

- [x] 3.1 Screenshots section: Add Screenshots… (native picker, multiple), listed names with remove, the skipped-files note
- [x] 3.2 l10n keys for every new label and message

## 4. Verification

- [x] 4.1 Unit tests: filtering/cap, body naming, delivery calls on both paths, no reveal without screenshots
- [x] 4.2 Dialog tests: add/remove/skipped through the entity, report carries the paths
- [x] 4.3 `make` green
