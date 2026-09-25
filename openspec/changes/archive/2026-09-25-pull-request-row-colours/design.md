# Design

## Precedence

A pull request can be several things at once, and a row has one colour. The
reasons are checked in the order the user has to act on them:

1. Conflicting (`mergeable: CONFLICTING` or `mergeStateStatus: DIRTY`) -
   nothing else matters until it is resolved.
2. Blocked by a draft or failing checks - not about to be merged whatever the
   API says.
3. Behind (`BEHIND`) - updating the branch reruns the checks, so it outranks
   checks running.
4. Checks running - GitHub reports `BLOCKED` while required checks are
   pending, so this is checked before `BLOCKED`; waiting is not a problem.
5. Blocked (`BLOCKED`) - typically a required review.
6. Mergeable (`mergeable: MERGEABLE`), otherwise unknown and uncoloured.

A `gh` that does not send `mergeStateStatus` loses only "behind" and the
`BLOCKED` case; the rest is decided from `mergeable` and the check rollup as
before.

## Colours

Orange covers both conflicts and the remaining blocks. Each needs a push or a
decision before the pull request can land, which is what the colour has to
say, and the palette has no further colour that stays distinct from the other
five states.
