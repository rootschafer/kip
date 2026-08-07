# Cloud Mirror Mode — Design TODO

**Status:** Paused. Design is settled; implementation is partly done and known-buggy.
**Supersedes:** nothing. Runs alongside the GUI work in `NEXT_AGENT_HANDOFF.md`.

## Goal

Back up directly into a cloud remote that mirrors the local filesystem layout —
no zip intermediate, no restructuring. A source at `$HOME/.ssh` should land at
`<remote>:<mirror_root>/.ssh`, preserving its path relative to `$HOME`.

The motivating case is a WebDAV/Nextcloud remote reached through rclone, where
the local disk has no room to stage a copy first. Nothing in the design is
Nextcloud-specific; any rclone remote works.

## Settled decisions

### 1. Mirror is a flag on `Destination`

Add `mirror: bool` to `Destination` (`cli/src/folder.rs`). When `mirror = true`,
kip computes the destination path by mirroring the source's path relative to
`$HOME`; the `path` field is ignored. Existing configs with an explicit `path`
keep working unchanged.

```toml
[[folders]]
source = "~/.ssh"
destinations = [
    { drive = "cloud", mirror = true },   # → <remote>:<mirror_root>/.ssh/
]
```

`mirror = true` and an explicit `path` are mutually exclusive — error if both
are set, rather than silently preferring one.

### 2. Mirror root is drive configuration, not a constant

The root the mirror hangs off is the cloud drive's `rclone_path`. It must not be
hardcoded anywhere: different machines mirroring into the same remote need
different roots, and a wrong root silently scatters files.

## Implementation status

### Phase A — direct (non-zip) cloud sync — PARTLY DONE, BUGGY

`backup_to_cloud_direct` and `backup_to_cloud_zipped` exist in `cli/src/backup.rs`
and are wired into the `DriveType::Cloud` branch.

**Known bug:** both functions build their `CloudDestination` from the *drive's*
`rclone_path` only and pass `""` as the subpath. The `dest_path` argument — which
carries the per-destination path — is accepted, logged, and then never used. As
written, every folder syncs to the same remote root and overwrites the last one.

Fix before anything else: thread `dest_path` into the `CloudDestination` so each
destination lands where its config says. Then test with two folders pointed at
the same drive and confirm they land in separate directories.

### Phase B — mirror mode — NOT STARTED

1. Add `mirror: bool` (default false) to `Destination`.
2. Add `Destination::resolved_path(source: &Path, drive_root: &str) -> Result<String>`:
   - `mirror == false` → return the explicit `path`.
   - `mirror == true` → strip `$HOME` from the absolute source path and join the
     remainder onto `drive_root`.
   - Error if the source is outside `$HOME`, rather than guessing.
3. Call `resolved_path` from the cloud branch of `backup.rs`.
4. Reject configs setting both `mirror` and `path`.

### Phase C — migrate existing app configs — NOT STARTED

Existing `apps/*.toml` use ad-hoc paths like `path = "identity"`. Switch them to
`mirror = true` once A and B are proven.

### Phase D — git awareness — DEFERRED

For sources that are git working trees: skip tracked files (they're in the
remote already), warn on uncommitted/unpushed changes, and back up gitignored
files *except* regeneratable build output.

Default skip list, which should be user-configurable (e.g.
`~/.config/kip/regeneratable.toml`):

- Rust: `target/`
- Node: `node_modules/`, `.next/`, `dist/`, `.cache/`
- Python: `__pycache__/`, `.venv/`, `venv/`, `*.egg-info/`, `.pytest_cache/`, `.mypy_cache/`, `.ruff_cache/`
- Other: `.gradle/`, `build/`, `out/`, `.terraform/`

### Phase E — retire the flat backup layout — NOT STARTED

Once mirroring is solid, migrate the older flat output structure over, or treat
the non-mirrored drive as a backup of the mirrored state.

## Files to study

| File | Why |
|---|---|
| `cli/src/backup.rs` (`DriveType::Cloud` branch, `backup_to_cloud_*`) | Where the Phase A bug lives. |
| `cli/src/folder.rs` | `Destination` / `FolderConfig` — where `mirror` goes. |
| `cli/src/drive_config.rs` | `DriveConfig`, `DriveType`, and the strict path resolution. |
| `crates/kip-rclone/src/rclone.rs` | `Rclone::copy` / `Rclone::sync`. |
| `crates/kip-rclone/src/destination.rs` | `CloudDestination`. |
| `examples/drives-with-cloud.toml` | Cloud drive config shape. |
| `docs/CLOUD_INTEGRATION.md` | Cloud setup notes. |

## Setup required to test

An rclone remote must exist and be reachable; configure it with `rclone config`
and confirm with `rclone listremotes` / `rclone lsd <remote>:`. Credentials live
in rclone's own config (`~/.config/rclone/rclone.conf`) — kip never stores them.

The rclone integration tests are `#[ignore]`d for exactly this reason; run them
with `cargo test -p kip-rclone -- --ignored` once a remote named `test_remote`
exists.

## Out of scope

- Restore from cloud (`DriveType::Cloud` in `cli/src/restore.rs` is a stub).
- Multi-drive mirroring.
- SurrealDB schema changes — the existing model already covers this.

## Definition of done

- [ ] Phase A bug fixed: two folders on one cloud drive land in distinct paths.
- [ ] `{drive = "...", mirror = true}` lands files at the mirrored path with no
      local zip intermediate.
- [ ] Verified with at least 3 source paths of different sizes.
- [ ] `mirror` + explicit `path` is rejected with a clear error.
- [ ] No regressions in the local and SSH backup paths.
