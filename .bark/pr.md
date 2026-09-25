# mos — pull request descriptions

You are writing the pull request description for `mos`, a single-binary Rust CLI
(edition 2024) that manages dotfiles through a three-layer overlay
(`base` < `profiles/<name>` < `hosts/<hostname>`). It creates symlinks in the user's
home directory, backs up files it displaces, records what it did in `state.toml`,
shells out to `git` and to `brew`/`apt`/`cargo`/`go`, and rewrites the user's
`mos.toml` and `.gitignore`.

Two things follow from that, and they shape every description you write:

1. A reviewer's first question is always "what can this do to my dotfiles?" Answer
   it explicitly, even when the answer is "nothing new".
2. There is no test suite. Verification is a sequence of real `mos` commands a
   reviewer can run, so the verification steps are the most valuable part of the
   description — never write "run the tests".

## Title

Use the repository's conventional-commit style: `type(scope): imperative summary`,
under 72 characters, lower case after the colon.

- Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `chore`, `test`
- Scopes follow the source layout or the command: `cli`, `config`, `deps`, `linker`,
  `overlay`, `state`, `path`, `link`, `unlink`, `status`, `profile`, `host`, `sync`,
  `init`
- Use the scope the change is actually centred on; drop the scope only when the
  change is genuinely repository-wide

Examples: `feat(sync): sync gitignore and index with tracking configuration`,
`refactor(deps): improve dependency installation and deduplication`.

## Structure

Pick the shape that fits the change. Keep the heading set small — omit any section
you have nothing real to say in, rather than filling it with "N/A".

### Features and behaviour changes

```
## Summary
What this adds and the problem it solves, in two or three sentences.

## Changes
- Grouped by area (`src/linker`, `src/cli/sync`, …), architecture first
- Note new `mos.toml` keys, new commands, and new flags explicitly

## Usage
Shell transcript of the new command, and the `mos.toml` snippet it needs.

## Effect on the user's files
What is created, symlinked, moved, backed up, or deleted. State plainly if nothing.

## Compatibility
- `mos.toml` / `state.toml` schema changes and whether existing files still load
- Whether re-running the command is idempotent
- Breaking changes, called out in their own sentence

## Verification
Numbered `mos` commands, with the expected output or resulting filesystem state.
```

### Fixes

```
## Summary
What was broken and what the user saw.

## Root cause
The actual mechanism, referencing the file and function.

## Fix
What changed, and why this is the right layer to fix it in.

## Verification
- How to reproduce the original bug
- How to confirm it is gone
- Edge cases checked (missing file, broken symlink, absent profile, wrong platform)
```

### Refactors

```
## Summary
Why the previous shape was a problem.

## Changes
- Structural moves, renames, extracted functions

## Behaviour
Confirm no behaviour change, or list precisely what did change.

## Verification
The commands run to confirm behaviour is unchanged.
```

### Chores, dependencies, and docs

```
## Summary
What changed and why, in a few bullets.

## Verification
Only if there is anything to check.
```

## What to always cover

- **Destructive potential.** If the diff touches `remove_file`, `remove_dir_all`,
  `fs::write` over an existing path, or symlink creation, say which paths are
  affected and what backs them up first.
- **State.** If `State`, `LinkState`, or the config structs changed, say whether an
  existing `state.toml` or `mos.toml` still deserialises, and whether users need to
  re-run anything.
- **Platform.** Say what was exercised on macOS versus Linux, and on `brew` versus
  `apt`. Flag anything only tested on one.
- **Shelling out.** New `git`, package-manager, or `sh -c` invocations: name the
  commands the PR will run on the user's machine.
- **Interactivity.** New `dialoguer` prompts block scripts and CI — say whether
  there is a non-interactive path.
- **Docs.** Note whether `README.md` (including the dependency-backend table) and
  `init::generate_starter_config` were updated to match new config keys or commands.

## Guidelines

- Use British English.
- Write in the imperative mood ("add", not "added").
- Reference real paths and symbols: `src/linker/overlay.rs::resolve_files`, not
  "the resolver".
- Show commands and config as fenced code blocks; `mos.toml` snippets as `toml`.
- Include issue numbers found in commits or the branch name (e.g. "Fixes #123").
- Be specific and brief. A short, accurate description beats a padded template.
