# tree-view

tree-view is a terminal filesystem browser. It loads only the directory where
it was started. Press `h` to load and move to its parent; pressing `h` again
loads the next parent. Select an entry with `j`/`k`, use `l` to enter it, and
then use `q` to quit there; pressing `q` while merely selecting a directory
does not enter it.

All keyboard bindings are stored in `config.toml`. The default navigation uses
Vim keys: `h`/`j`/`k`/`l`. Edit the values to use arrows or WASD:

```toml
up = "k"
down = "j"
left = "h"
right = "l"
```

The configuration is reloaded automatically when a key is pressed, so changes
take effect without recompiling or restarting the program.

When running the program from another directory, use
`~/.config/tree-view/config.toml` or set `TREE_VIEW_CONFIG` to the desired file.
Arrow names (`up`, `down`, `left`, `right`) and special names (`enter`, `esc`,
`backspace`) are accepted as values. Every action must be configured, and no
two actions may use the same key. The commented WASD example uses `x` for
delete because `d` is needed for right movement.

To make `q` leave the shell in the selected destination, source the wrapper and
use the `kd` function instead of invoking the binary directly:

```bash
cargo build
```

Run `cargo build` once (or after changing the code).
The wrapper automatically locates `target/debug/tree-view` (preferred during
development), then `target/release/tree-view`.

## Bash

```bash
  source /absolute/path/to/tree-dir/scripts/kd.sh
kd
```

To configure it permanently in Bash:

```bash
echo 'source /absolute/path/to/tree-dir/scripts/kd.sh' >> ~/.bashrc
```

## Fish

Load the Fish wrapper and run the `kd` function:

```fish
source /absolute/path/to/tree-dir/scripts/kd.fish
kd
```

To configure it permanently in Fish:

```fish
echo 'source /absolute/path/to/tree-dir/scripts/kd.fish' >> ~/.config/fish/config.fish
```

## Zsh

The Bash wrapper uses `${BASH_SOURCE[0]}`, so it should not be loaded in Zsh.
In Zsh, use the dedicated wrapper:

```zsh
source /absolute/path/to/tree-dir/scripts/kd.zsh
kd
```

Do not run `./scripts/kd.zsh` directly. The file defines a `kd` function,
and only a function loaded into the current shell can change that shell's
working directory.

To configure it permanently in Zsh:

```zsh
echo 'source /absolute/path/to/tree-dir/scripts/kd.zsh' >> ~/.zshrc
```

Then run `source ~/.zshrc` or open a new terminal. Do not run
`./target/debug/tree-view` or `cargo run` directly if you want the shell to
change directories: only the `kd` function can run `cd` in the parent
shell. A standalone binary cannot change its parent shell's directory on
Unix-like systems.

After changing the code, run `cargo build` and reload the wrapper if needed:

```zsh
cargo build
source ~/Desktop/tree-dir/scripts/kd.zsh
kd
```
