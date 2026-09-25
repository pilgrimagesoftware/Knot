# Tasks

## 1. Forge

- [x] 1.1 Add `ForgeError::NotFound` and `consts::NOT_FOUND_MARKERS`; classify a
      matching `ForgeError::Command` in `pull_request.rs`'s `fetch`. Tests
      through the stub runner for both messages, for case-insensitivity, and
      for an unrelated command failure staying `Command`.

## 2. Cache

- [x] 2.1 Add `PullRequestLookup` (known, not found, failed) with `state()` and
      `is_final()` and a conversion from the forge result; make it the
      `PullRequestStateCache` value. Update `expired_urls` and its tests.
- [x] 2.2 Add `RefreshCache::holds`, tested.
- [x] 2.3 Skip final lookups in `refresh_pull_request_states`.

## 3. Counts

- [x] 3.1 Add `not_found` to `PullRequestCounts` (in `total`, known for
      `nothing_known`), count it in `counts_for`, and show
      `pull_requests.count_not_found` in `counts_label` when non-zero. Tests.

## 4. Row

- [x] 4.1 Carry the lookup into `PullRequestRow`; add `RowStatus` for the icon
      and detail line; give not found its own icon and the
      `pull_requests.not_found` label. Tests for the icon being unique and the
      keys resolving.

## 5. Gate

- [x] 5.1 `openspec validate pull-request-not-found --strict` passes.
- [x] 5.2 `make` passes.
