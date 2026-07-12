# Plan 005: Validate and atomically publish migration artifacts

> **Executor instructions**: Follow every step and verification. Stop and
> report instead of widening scope. Update the 005 row in `plans/README.md`
> when done.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- cli/src/commands/generate.rs migrations/src/writer.rs migrations/src/words.rs migrations/src/dir.rs plans/README.md`
> Semantic drift from the excerpts below is a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: `plans/001-verify-compile-fail-diagnostics.md`
- **Category**: migration
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

Custom migration names become path components without validation, and both
writers create the final folder before all files are durable. A separator or
parent component can escape the output root; a collision can overwrite an
existing migration; a failed second write leaves a partial folder. Validate
the component and publish a complete staged directory with a no-replace rename.

## Current state

- `migrations/src/words.rs:1467-1487` copies `custom_name` verbatim into the
  generated tag.
- `cli/src/commands/generate.rs:208-233` does:

```rust
let migration_dir = out_dir.join(tag);
std::fs::create_dir_all(&migration_dir)?;
std::fs::write(migration_dir.join("migration.sql"), sql_content)?;
snapshot.save(migration_dir.join("snapshot.json"))?;
```

- `write_custom_migration` at `cli/src/commands/generate.rs:246-263` repeats the
  same direct-final-folder pattern.
- Low-level `Writer::write_sqlite_migration` and `Writer::write_custom_migration`
  repeat it at `migrations/src/writer.rs:222-246` and `:300-323`.
- `MigrationError` already carries `ConfigError`, `IoError`, and
  `SnapshotError`; adding a breaking return type is unnecessary.
- `MigrationDir::discover_v3` currently ignores unreadable directory entries
  with `filter_map(Result::ok)`; that separate fail-closed work belongs to plan
  006, not this plan.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Writer tests | `cargo test -p drizzle-migrations writer` | all pass |
| CLI generate tests | `cargo test -p drizzle-cli --features rusqlite generate` | all pass |
| Full migration crate | `cargo test -p drizzle-migrations` | all pass |
| Format | `cargo +nightly fmt --all -- --check` | exit 0 |
| Lint | `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

## Scope

**In scope**:

- `migrations/src/words.rs` for a reusable component validator and tests
- `migrations/src/writer.rs` for staged publication and tests
- `cli/src/commands/generate.rs` for validation, staged publication, and tests
- `cli/src/error.rs` only if a clearer existing-error mapping is needed
- `plans/README.md` (status row only)

**Out of scope**:

- Changing the tag format for already valid names
- Renaming or rewriting existing migrations
- Journal/discovery behavior
- Cross-process migration execution locking
- Database transactions or migration SQL semantics
- A breaking change to `generate_migration_tag*` or `Writer` public signatures

## Git workflow

- Branch: `codex/005-atomic-migration-files`
- Suggested commits: `fix: validate migration path components`, then
  `fix: publish migration folders atomically`
- Do not push or open a PR unless asked.

## Steps

### Step 1: Define one migration-name component policy

Add a non-breaking validator in `migrations/src/words.rs` returning a typed or
string-backed configuration error. Accept the existing ordinary names,
Unicode, spaces if already supported, hyphens, and underscores. Reject empty or
whitespace-only names, `.` and `..`, absolute/rooted/prefixed paths, either
platform's separators (`/` and `\\`), NUL/control characters, and any value
that is not exactly one `std::path::Component::Normal` component. Also reject
names whose generated full tag is not a single normal component.

Call it at both filesystem boundaries before any directory is created. Keep
the existing public tag generators returning `String`; validation belongs to
writers/CLI so this plan does not break callers that only format tags.

**Verify**: focused `words` tests accept representative current names and reject
all listed invalid categories on Windows and Unix parsing rules.

### Step 2: Add staged directory publication

For each writer, create a uniquely named temporary sibling directory under the
same output root, write every required file there, flush/close file handles,
then rename the directory to the final tag path. Same-parent staging keeps the
rename on one filesystem. Use `create_dir`, not `create_dir_all`, for both the
unique staging leaf and final collision semantics.

Before publish, fail if the final path exists. Do not remove or overwrite that
path. On any write/snapshot/rename error, best-effort remove only the verified
temporary sibling created by this invocation; never recursively remove the
final path. On Windows, treat an existing destination rename error as a
collision rather than deleting/retrying.

Share a private helper within each crate if needed; do not introduce a large
public filesystem abstraction merely to remove a few duplicate lines.

**Verify**: writer and CLI focused tests pass.

### Step 3: Add failure and collision tests

Using `tempfile`, test:

1. valid generated and custom migrations contain all required files;
2. invalid names create nothing outside or inside the output root;
3. an existing final tag is unchanged and returns an error;
4. an injected/forced second-file failure leaves no final folder and no staging
   folder (factor writing behind a private closure/helper to make this
   deterministic rather than relying on filesystem permissions);
5. two attempts at the same tag produce exactly one complete winner.

Do not use sleeps or timestamp luck; pass a deterministic tag into the private
publication helper in tests.

**Verify**: both focused test commands pass twice consecutively.

### Step 4: Run full gates and inspect filesystem operations

Run the full migration tests, fmt, and clippy. Search the two writer files for
direct final-folder creation and ensure it occurs only inside the safe publish
helper, if at all.

**Verify**: all commands exit 0; `git diff --check` exits 0.

## Test plan

- Cross-platform component validation, including both separator styles.
- Successful SQL+snapshot publication and custom SQL publication.
- Existing destination remains byte-for-byte unchanged.
- Deterministic mid-write failure leaves no visible partial migration.
- No staging directory remains after success or failure.

## Done criteria

- [ ] Both CLI and low-level writer validate before filesystem mutation.
- [ ] Existing migration folders are never overwritten.
- [ ] Final folders become visible only after all required files are written.
- [ ] Cleanup is limited to this invocation's verified staging directory.
- [ ] Existing public signatures and valid tag shapes are preserved.
- [ ] Focused, full, fmt, and clippy commands exit 0.
- [ ] `plans/README.md` marks 005 DONE.

## STOP conditions

- Plan 001 is not DONE.
- A proposed cleanup path cannot be proven to be a direct child of the output
  root.
- Atomic same-filesystem rename is unavailable on a supported platform.
- Correct collision handling would require overwriting/deleting an existing
  final migration.
- Preserving valid names requires changing a public return type.

## Maintenance notes

Reviewers should focus on path containment and cleanup ownership. Future files
added to a migration folder must be written into staging before publication,
and discovery must continue to see only complete final directories.
