# Controller decisions made during implementation

These decisions were taken on the operator's behalf while executing the OpenWatch plan.
Each was recorded in the SDD ledger at the time; this file preserves them after workspace cleanup.

1. **Shared UI crate** — Android cannot compile against a binary crate. `crates/ui` is a library
   holding all `.slint` files and `AppState`; `crates/desktop` and `crates/android` are thin hosts
   depending on it. Cost if wrong: later UI rework.

2. **Worktree via git fallback** — the session root is not a git repo, so the native worktree tool
   would have targeted the wrong repository. Isolation used `git worktree` with absolute paths.
   Cost if wrong: the harness does not track this worktree.

3. **Live sweep pending hardware** — the live verification harness is `#[ignore]` and env-gated;
   the actual hardware pass is pending because `hci0` was DOWN and the watch was not advertising.
   Treated as an environment limitation, not a fix-loop defect. Cost if wrong: opcode promotions
   rest on existing captures rather than a fresh run.

4. **UI-freeze item reclassified, not fixed** — off-thread Slint refresh requires `!Send` window
   marshaling beyond the final fix wave. The earlier "ADDRESSED" claim was withdrawn and the item
   is honestly recorded as NOT ADDRESSED in the backlog. Cost if wrong: Sync still blocks the UI
   for roughly 7 seconds until a future worker-thread pass.
