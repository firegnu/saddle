#!/bin/sh
# Build an immutable product directory; never install, register plugins or switch links.
set -eu
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 NEW_OUTPUT_DIRECTORY" >&2
    exit 2
fi
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
case "$output" in /*) ;; *) output="$PWD/$output" ;; esac
if [ -e "$output" ] || [ -L "$output" ]; then
    echo "Refusing to overwrite an existing product directory: $output" >&2
    exit 1
fi
build_dir=${CARGO_TARGET_DIR:-"$project_dir/target"}
case "$build_dir" in /*) ;; *) build_dir="$PWD/$build_dir" ;; esac
host=$(rustc -vV | sed -n 's/^host: //p')
cd "$project_dir"
cargo build --bin saddle --release --locked --target "$host" --target-dir "$build_dir"
mkdir -p "$(dirname -- "$output")"
staging=$(mktemp -d "$(dirname -- "$output")/.saddle-package.XXXXXX")
trap 'rm -rf -- "$staging"' EXIT HUP INT TERM
mkdir -p "$staging/bin"
cp "$build_dir/$host/release/saddle" "$staging/bin/saddle"
{
    printf 'revision: '; git rev-parse HEAD
    printf 'target: %s\n' "$host"
    printf 'working-tree: '; if [ -z "$(git status --porcelain)" ]; then printf 'clean\n'; else printf 'modified\n'; fi
    # The Saddle source and branch this was built from, so Updates can compare it later.
    printf 'source: %s\n' "$project_dir"
    printf 'branch: '; git symbolic-ref --quiet --short HEAD || printf '\n'
    (cd "$staging" && shasum -a 256 bin/saddle)
} > "$staging/BUILD.txt"
if [ -e "$output" ] || [ -L "$output" ]; then
    echo "Output appeared during build; nothing installed" >&2
    exit 1
fi
mv "$staging" "$output"
trap - EXIT HUP INT TERM
printf 'Product directory: %s\nNo global installation or registration performed.\n' "$output"
