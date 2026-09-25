# mos — review guide

`mos` is a single-binary Rust CLI (edition 2024) that manages dotfiles through a
three-layer overlay (`base` < `profiles/<name>` < `hosts/<hostname>`). It creates
symlinks into the user's home directory, backs up files it displaces, records what
it did in `state.toml`, shells out to `git`, `brew`/`apt`/`cargo`/`go`, and rewrites
the user's `mos.toml` and `.gitignore`.

Every operation touches files the user cannot afford to lose. Prioritise
destructive-filesystem and state-consistency findings above everything else.

There is no async, no threading, no `unsafe`, and no public library API — do not
raise findings about those.

## Destructive filesystem operations (highest priority)

- Flag any `remove_file`, `remove_dir_all`, or overwrite of a path under `$HOME`
  that is not preceded by a check that the path is what we think it is (symlink vs
  regular file vs directory)
- Flag any path where a regular file could be deleted or overwritten without
  `backup_file` running first and succeeding — the backup-before-overwrite
  invariant in `linker::link_module` must never be bypassed
- Flag `remove_dir_all` on a path derived from user input (profile name, hostname,
  module source) without verifying it stays inside the dotfiles directory
- Flag operations that follow a symlink when they should act on the link itself —
  `exists()` follows links, `is_symlink()` does not; `exists()` is false for a
  broken symlink
- Flag new destructive paths that skip the `Confirm::with_theme(...).default(false)`
  prompt used by `profile remove` and `host remove`
- Flag `create_dir_all` / write paths that could clobber an existing user directory
  rather than extend it

## State consistency

- Flag code paths that mutate the filesystem but return early without reaching
  `state.save()` — `state.toml` must not claim links that do not exist, or omit
  links that do
- Flag partial-failure handling: when one module in a loop fails, say whether the
  already-linked modules are recorded (compare with the `?`-on-first-error pattern
  in `cli/link.rs` and `cli/sync.rs`)
- Flag `state.backups` entries written without the backup actually being created,
  or backups created without being recorded
- Flag changes to `State`, `LinkState`, `Config`, `Module`, `Profile`, or `Host`
  that break deserialisation of an existing on-disk `state.toml` or `mos.toml` —
  new fields need `#[serde(default)]` or a `default_*` fn
- Flag re-running a command that would not be idempotent: `mos link` twice must
  leave the same links, `.gitignore`, and state

## Overlay resolution

- Flag any change that breaks the documented precedence `host > profile > base`
- Flag file-mapping vs directory-mapping logic that diverges — `resolve_files`
  decides between them with `is_file()`; a module that is a file in one layer and a
  directory in another is a real case to handle
- Flag layer strings (`"base"`, `"profile/<name>"`, `"host/<hostname>"`) built
  inconsistently, since `status` and `state.toml` display them
- Flag `primary_layer` / "winning layer" logic that reports the wrong layer when
  files come from more than one layer
- Flag `WalkDir` traversal that silently drops entries with `filter_map(|e| e.ok())`
  where a permission error should surface, and traversal that ignores symlinked
  directories or hidden files it should include

## Paths and expansion

- Flag user-supplied paths used without `path::expand_path` — `~` and `$VAR`
  expansion must be applied consistently
- Flag duplicated tilde/env expansion instead of reusing `path::expand_path`
  (`cli/init.rs::expand_input_path` already duplicates it; do not add a third)
- Flag string concatenation of paths (`format!("{}/{}", ...)`) where `PathBuf::join`
  is correct
- Flag `display().to_string()` or `to_string_lossy()` used as a path key where a
  non-UTF-8 path would be silently corrupted, especially round-tripping through
  `state.toml`
- Flag missing handling for `XDG_CONFIG_HOME` / `XDG_DATA_HOME` when new files are
  read or written, and any config or state path that bypasses
  `State::state_path()` / `GlobalConfig::path()`

## Shelling out

- Flag command arguments built by splitting a string on whitespace — `commit_cmd`
  already does this and breaks on quoted arguments; do not extend the pattern
- Flag interpolation of config or user values into `sh -c` strings (`deps::script`)
  without noting the injection surface
- Flag `Command` invocations whose non-zero exit status is not checked, or whose
  stderr is discarded when it is the only clue to the failure
- Flag `git` calls that assume a clean tree, an existing remote, an upstream
  branch, or a repository at all
- Flag `git rm --cached` / `git add -A` argument lists built without `--` before
  user-derived paths
- Flag new interactive `dialoguer` prompts on a code path that CI or a script would
  hit — `mos sync push` without `-m` already blocks; prefer a flag with a prompt
  fallback

## `mos.toml` and `.gitignore` rewriting

- Flag config writes that go through `toml`/`serde` serialisation instead of
  `toml_edit`, which is what preserves the user's comments and formatting
- Flag `doc["..."]` indexing that panics on a missing or wrongly-typed table
  instead of `.as_table_mut().ok_or_else(...)`
- Flag config mutations written without `Config::validate()` still holding
  afterwards — dangling `default_profile` or profile-to-module references
- Flag `.gitignore` reconciliation that matches lines with exact string equality
  where a user's trailing whitespace, comment, or negation (`!pattern`) would be
  missed, and any rewrite that drops lines mos did not add
- Flag module-segment derivation (`source.split('/').next()`) applied to a source
  that is a bare file rather than a directory

## Cross-platform behaviour

- Flag macOS-only or Linux-only assumptions that are not behind `cfg` or a
  `PackageManager` check — brew paths, apt availability, `hostname` output
- Flag package resolution that adds a dependency to both `brew` and `apt` when only
  one manager is present, or silently drops a package when a `PackageDep::Platform`
  entry omits the current platform
- Flag missing deduplication for newly collected dependency kinds (`cargo`, `go`,
  and `script` are still not deduplicated in `CollectedDeps::from_profile`)
- Flag binary-name vs package-name confusion — `is_installed` checks the binary
  (`which`), install takes the package; `GoDep::bin()` / `CargoDep::bin()` exist for
  this reason

## Error handling

- Flag `.unwrap()` and `.expect()` in non-test code — use `?` with `eyre!` context
- Flag indexing that can panic: `config.modules[name]`, `config.profiles[name]`,
  `state.links[name]` — prefer `.get()` with a clear error
- Flag errors swallowed by `let _ = ...`, `.ok()`, or `ok()?` where the user needs
  to know something failed
- Flag propagated errors that lose context — follow the
  `.map_err(|e| eyre!("failed to <verb> {}: {}", path.display(), e))` convention
- Flag error messages that break house style: lower-case, no trailing full stop,
  path included via `.display()`, and an actionable next step where one exists
  (e.g. "run `mos init` first")

## Rust hygiene

- Flag unnecessary `.clone()` where borrowing works, especially cloning `Config`,
  `State`, or whole `Vec<String>` module lists
- Flag `String` parameters that should be `&str`, and `Vec<T>` that should be `&[T]`
- Flag `Vec` built in a loop with a known size and no `with_capacity`
- Flag repeated `Config::load` / `State::load` in one command run
- Flag `if let` chains and let-else that could replace nested matching — the
  codebase already uses edition-2024 `let` chains
- Flag pure logic (overlay precedence, dependency resolution, gitignore
  reconciliation, `GoDep::bin()` parsing) that is entangled with I/O and so cannot
  be tested; there are currently no tests, so extractability matters

## CLI surface and docs

- Flag new commands, subcommands, or flags whose clap doc comment is missing or
  unclear — those comments are the user-facing help text
- Flag `println!` output that departs from existing shape: two-space indent for
  per-item lines, `Module '<name>':` headers, `target -> source [layer]` for links
- Flag new `mos.toml` keys, commands, or dependency backends not reflected in
  `README.md` (including the backend table) and in `init::generate_starter_config`
- Flag output written to stdout that should be stderr, and failures that print a
  message but still exit zero
- Flag remnants of the project's former name — `init` still creates a `.dotm`
  directory; new code should use `mos` consistently

## Style

- Use British English in review comments.
- Anchor each finding to the concrete file it affects and say what breaks, not just
  which rule it violates.
