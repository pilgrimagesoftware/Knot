## 1. Sequencing

- [x] 1.1 Archive `agent-context-menu-parity`, whose implementation is merged,
      so the Open In requirement exists in `openspec/specs/agent-list-ui/`.
      Verify `openspec validate open-in-zed --strict` no longer reports that
      archive would refuse this delta.
      **Done by PR #131**, which archived the eight completed changes
      together; nothing was needed here.

## 2. The entry

- [x] 2.1 Add Zed to `open_in.rs`'s submenu list after VS Code, and
      `-b dev.zed.Zed` to its argument mapping. Verify with the existing
      tests, extended: the submenu order includes Zed in the editors group,
      and Zed maps to the bundle-id form rather than the application-name
      form.

## 3. Verification

- [x] 3.1 `make rust` passes clean.
- [ ] 3.2 In the app, select Zed from an agent's Open In… submenu and confirm
      that agent's folder opens in Zed.
