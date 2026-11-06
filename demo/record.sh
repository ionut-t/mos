#!/usr/bin/env bash
# Records assets/demo.gif with VHS inside a throwaway fake HOME.
#
# Everything lives in demo/.tmp (git-ignored). /tmp/mos-demo is a symlink to it
# so the paths shown on screen don't include the real home directory. No
# network access, and nothing reads the real ~/.dotfiles or ~/.config.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$REPO/demo/.tmp"
LINK=/tmp/mos-demo

cleanup() {
    [ -L "$LINK" ] && rm -f "$LINK"
    rm -rf "$TMP"
}

if [ -e "$LINK" ] && [ ! -L "$LINK" ]; then
    echo "error: $LINK exists and is not a symlink; refusing to touch it" >&2
    exit 1
fi

cleanup
trap cleanup EXIT

echo "==> building mos"
cargo build --release --quiet --manifest-path "$REPO/Cargo.toml"

mkdir -p "$TMP"
ln -s "$TMP" "$LINK"

# Minimal PATH for the recorded shell: only these tools plus /usr/bin:/bin.
mkdir -p "$TMP/bin"
ln -s "$REPO/target/release/mos" "$TMP/bin/mos"
for tool in tree; do
    if command -v "$tool" >/dev/null; then ln -s "$(command -v "$tool")" "$TMP/bin/$tool"; fi
done

# Isolated environment for every git call made while seeding.
seed_env() {
    env -i PATH=/usr/bin:/bin HOME="$LINK/$1" GIT_CONFIG_NOSYSTEM=1 \
        GIT_AUTHOR_NAME="Demo User" GIT_AUTHOR_EMAIL=demo@example.com \
        GIT_COMMITTER_NAME="Demo User" GIT_COMMITTER_EMAIL=demo@example.com \
        "${@:2}"
}

echo "==> seeding fake machines"
LAPTOP="$LINK/laptop"
DOT="$LAPTOP/.dotfiles"
mkdir -p "$DOT"/{base/zsh,base/git,base/nvim/lua,profiles/work/git,hosts/desktop/nvim/lua} \
    "$LINK/desktop"

cat >"$DOT/mos.toml" <<'EOF'
[settings]
dotfiles_dir = "~/.dotfiles"

[hosts]
laptop  = { os = "macos", default_profile = "home" }
desktop = { os = "linux", default_profile = "home" }

[profiles]
home = { modules = ["zsh", "git", "nvim"] }
work = { modules = ["zsh", "git", "nvim"] }

[modules]
zsh  = { source = "zsh/.zshrc",     target = "~/.zshrc" }
git  = { source = "git/.gitconfig", target = "~/.gitconfig" }
nvim = { source = "nvim",           target = "~/.config/nvim" }
EOF

cat >"$DOT/base/zsh/.zshrc" <<'EOF'
export EDITOR=nvim
alias ll="ls -lah"
alias g="git"
EOF

cat >"$DOT/base/git/.gitconfig" <<'EOF'
[user]
    name = Demo User
    email = demo@example.com
[init]
    defaultBranch = main
EOF

cat >"$DOT/profiles/work/git/.gitconfig" <<'EOF'
[user]
    name = Demo User
    email = demo@work.example.com
[init]
    defaultBranch = main
EOF

cat >"$DOT/base/nvim/init.lua" <<'EOF'
require("options")
EOF

cat >"$DOT/base/nvim/lua/options.lua" <<'EOF'
vim.opt.number = true
vim.opt.expandtab = true
vim.opt.shiftwidth = 4
EOF

cat >"$DOT/hosts/desktop/nvim/lua/options.lua" <<'EOF'
vim.opt.number = true
vim.opt.relativenumber = true
vim.opt.expandtab = true
vim.opt.shiftwidth = 2
EOF

printf '\n' >"$DOT/.gitignore"

# A pre-existing, unmanaged ~/.gitconfig on the laptop so `mos link` backs it up.
cp "$DOT/base/git/.gitconfig" "$LAPTOP/.gitconfig"

# Local bare repo acting as the remote.
seed_env laptop git init -q --bare -b main "$LINK/remote.git"
seed_env laptop git -C "$DOT" init -q -b main
seed_env laptop git -C "$DOT" add -A
seed_env laptop git -C "$DOT" commit -q -m "initial dotfiles"
seed_env laptop git -C "$DOT" remote add origin "$LINK/remote.git"
seed_env laptop git -C "$DOT" push -q -u origin main

# rc files for the recorded shells: clean env, fake HOME, neutral prompt.
write_rc() {
    local name="$1" start="$2"
    cat >"$TMP/$name.rc" <<EOF
export HOME=$LINK/$name
export XDG_CONFIG_HOME="\$HOME/.config" XDG_DATA_HOME="\$HOME/.local/share" XDG_CACHE_HOME="\$HOME/.cache"
export MOS_HOSTNAME=$name
export PATH=$LINK/bin:/usr/bin:/bin
export GIT_CONFIG_NOSYSTEM=1 HISTFILE=/dev/null CLICOLOR=1 LANG=en_US.UTF-8
export BASH_SILENCE_DEPRECATION_WARNING=1
PS1='\[\e[2m\]$name\[\e[0m\] \[\e[1;35m\]❯\[\e[0m\] '
cd "$start"
clear
EOF
}
write_rc laptop "$DOT"
write_rc desktop "$LINK"

echo "==> recording"
mkdir -p "$REPO/assets"
(cd "$REPO" && vhs demo/demo.tape)

echo "==> wrote assets/demo.gif"
