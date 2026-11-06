# mos demo — run via demo/record.sh (it builds the fake HOME this tape expects).

Output assets/demo.gif

Set Shell "bash"
Set Theme "Catppuccin Mocha"
Set FontFamily "JetBrains Mono"
Set FontSize 16
Set Width 1500
Set Height 720
Set Padding 20
Set TypingSpeed 60ms

# Hidden setup: swap into a clean environment with the fake laptop HOME.
Hide
Type "exec env -i TERM=xterm-256color /bin/bash --noprofile --rcfile /tmp/mos-demo/laptop.rc -i"
Enter
Sleep 1s
Show

# 1. Layout: base < profile < host
Type "tree -aC -I .git"
Sleep 100ms
Enter
Sleep 1.5s

# 2. Link + status
Hide
Type "clear"
Enter
Show
Type "mos link && mos status"
Sleep 100ms
Enter
Sleep 2.1s

# 3. Profile overlay
Hide
Type "clear"
Enter
Show
Type "mos profile switch work && git config user.email"
Sleep 100ms
Enter
Sleep 1.5s

# 4. Edit through the symlink and push
Type `echo "alias gs='git status'" >> ~/.zshrc && mos sync push -m "gs alias"`
Sleep 100ms
Enter
Sleep 1.6s

# 5. Fresh machine: clone + init + link
Hide
Type "exec env -i TERM=xterm-256color /bin/bash --noprofile --rcfile /tmp/mos-demo/desktop.rc -i"
Enter
Sleep 1s
Show
Type "git clone -q remote.git ~/.dotfiles && mos init"
Sleep 100ms
Enter
Sleep 500ms
Enter
Sleep 400ms
Enter
Sleep 1s
Hide
Type "clear"
Enter
Show
Type "mos link && mos status"
Sleep 100ms
Enter
Sleep 2.5s
