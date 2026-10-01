#!/usr/bin/env bash

treenav() {
    local destination
    local script_dir
    local binary
    local cwd_file
    script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)" || return
    if [[ -x "$script_dir/target/debug/tree-view" ]]; then
        binary="$script_dir/target/debug/tree-view"
    elif [[ -x "$script_dir/target/release/tree-view" ]]; then
        binary="$script_dir/target/release/tree-view"
    elif command -v tree-view >/dev/null 2>&1; then
        binary="$(command -v tree-view)"
    else
        printf 'tree-view not found. Run: cargo build\n' >&2
        return 127
    fi

    cwd_file="$(mktemp "${TMPDIR:-/tmp}/treenav-cwd.XXXXXX")" || return
    if ! TREENAV_CWD_FILE="$cwd_file" "$binary" "$@"; then
        rm -f -- "$cwd_file"
        return 1
    fi
    destination="$(cat -- "$cwd_file")"
    rm -f -- "$cwd_file"
    if [[ -d "$destination" ]]; then
        builtin cd -- "$destination"
    else
        printf 'tree-view did not return a valid directory: %s\n' "$destination" >&2
        return 1
    fi
}
