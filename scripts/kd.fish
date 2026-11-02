#   source /absolute/path/to/tree-dir/scripts/kd.fish

function kd
    set -l script_dir (cd -- (dirname -- (status filename))/..; and pwd)
    or return

    set -l binary
    if test -x "$script_dir/target/debug/tree-view"
        set binary "$script_dir/target/debug/tree-view"
    else if test -x "$script_dir/target/release/tree-view"
        set binary "$script_dir/target/release/tree-view"
    else if command -q tree-view
        set binary (command -s tree-view)
    else
        printf 'tree-view not found. Run: cargo build\n' >&2
        return 127
    end

    set -l tmpdir /tmp
    if set -q TMPDIR[1]
        set tmpdir "$TMPDIR"
    end
    set -l cwd_file (mktemp "$tmpdir/kd-cwd.XXXXXX")
    or return

    if not env KD_CWD_FILE="$cwd_file" "$binary" $argv
        rm -f -- "$cwd_file"
        return 1
    end

    set -l destination (cat -- "$cwd_file")
    rm -f -- "$cwd_file"
    if test -d "$destination"
        cd -- "$destination"
    else
        printf 'tree-view did not return a valid directory: %s\n' "$destination" >&2
        return 1
    end
end
