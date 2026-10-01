# tree-view

tree-view is a terminal filesystem browser. It loads only the directory where
it was started. Press `h` to load and move to its parent; pressing `h` again
loads the next parent. Select an entry with `j`/`k`, use `l` to enter it, and
then use `q` to quit there; pressing `q` while merely selecting a directory
does not enter it.

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

## Zsh

The Bash wrapper uses `${BASH_SOURCE[0]}`, so it should not be loaded in Zsh.
In Zsh, use the dedicated wrapper:

```zsh
source /absolute/path/to/tree-dir/scripts/kd.zsh
kd
```

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
