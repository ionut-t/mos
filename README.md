# mos

Dotfiles manager with multi-machine, multi-profile overlay support.

## Install

```sh
cargo install --git https://github.com/ionut-t/mos
```

## How it works

Files live in a three-layer structure inside your dotfiles repo:

```
~/.dotfiles/
  base/          # shared across all machines and profiles
  profiles/      # profile-specific overrides
  hosts/         # machine-specific overrides
  mos.toml       # config
```

When linking, `host` beats `profile` beats `base`. The winning file is symlinked to its target.

## Setup

```sh
mos init          # creates the directory structure and mos.toml
```

## Usage

```sh
mos link                      # link all modules in the active profile
mos link <module>             # link a single module
mos unlink <module>           # remove symlinks for a module
mos status                    # show linked modules and drift

mos profile list              # list profiles
mos profile switch <name>     # switch active profile
mos profile create <name>     # create a new profile interactively
mos profile override <module> # copy base files into the active profile for editing

mos host list                 # list registered hosts
mos host register             # register this machine interactively
mos host info                 # show this host's config

mos deps check                # show which dependencies are installed
mos deps install              # install missing dependencies
```

## mos.toml

```toml
[settings]
dotfiles_dir = "~/.dotfiles"
backup_dir   = "~/.mos-backups"

[hosts."my-mac.local"]
os              = "macos"
package_manager = "brew"
default_profile = "home"

[hosts."my-linux.local"]
os              = "linux"
package_manager = "apt"
default_profile = "home"

[profiles.home]
modules = ["zsh", "nvim", "git"]

[modules.zsh]
source  = "zsh/.zshrc"
target  = "~/.zshrc"

[modules.zsh.deps]
packages = ["fzf", "eza", "zoxide"]

# deps-only module (no source/target)
[modules.ripgrep.deps]
packages = ["ripgrep"]

# cross-platform with name differences
[modules.yazi.deps]
packages = [
  "yazi",
  "ffmpeg",
  { brew = "sevenzip", apt = "7zip" },
  { brew = "poppler",  apt = "poppler-utils" },
  { brew = "fd",       apt = "fd-find" },
  "ripgrep",
  "fzf",
  { brew = "resvg" },   # not available on apt
]
```

## Dependency backends

| Backend    | Example                                              |
| ---------- | ---------------------------------------------------- |
| `packages` | `["git"]` or `[{ brew = "sevenzip", apt = "7zip" }]` |
| `brew`     | `["git"]` or `[{ pkg = "ripgrep", bin = "rg" }]`     |
| `apt`      | `["git"]` or `[{ pkg = "fd-find", bin = "fdfind" }]` |
| `cargo`    | `["stylua"]`                                         |
| `go`       | `["github.com/user/tool@latest"]`                    |
| `script`   | `[{ name = "x", cmd = "curl ..." }]`                 |

`packages` is the cross-platform field. Plain strings use the same name on every manager; inline maps let you specify per-manager names or omit a key to skip that platform. `brew` and `apt` are for packages that only make sense on one platform.
