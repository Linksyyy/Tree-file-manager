# Load this file from zsh with:
#   source /absolute/path/to/tree-dir/scripts/treenav.zsh
typeset -g TREENAV_SCRIPT_DIR="${${(%):-%x}:A:h:h}"

treenav() {
    local destination
    local binary
    local cwd_file
    if [[ -x "$TREENAV_SCRIPT_DIR/target/debug/tree-view" ]]; then
        binary="$TREENAV_SCRIPT_DIR/target/debug/tree-view"
    elif [[ -x "$TREENAV_SCRIPT_DIR/target/release/tree-view" ]]; then
        binary="$TREENAV_SCRIPT_DIR/target/release/tree-view"
    elif (( $+commands[tree-view] )); then
        binary="$commands[tree-view]"
    else
        print -u2 'tree-view not found. Run: cargo build'
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
        print -u2 -- "tree-view did not return a valid directory: $destination"
        return 1
    fi
}
