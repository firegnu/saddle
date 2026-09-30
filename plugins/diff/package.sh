#!/bin/sh
# Build one local plugin directory. Does not register it or modify the host.
set -eu
if [ "$#" -gt 1 ]; then
    echo "Usage: $0 [output-directory]" >&2
    exit 2
fi
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir=${1:-"$project_dir/dist/diff-plugin"}
build_dir=${CARGO_TARGET_DIR:-"$project_dir/target"}
host=$(rustc -vV | sed -n 's/^host: //p')
cargo install --path "$project_dir" --bin saddle-diff --locked \
    --root "$output_dir" --target-dir "$build_dir" --target "$host" \
    --no-track --force
cp "$project_dir/plugin.toml" "$output_dir/plugin.toml"
output_dir=$(CDPATH= cd -- "$output_dir" && pwd)
printf '\nPlugin directory: %s\n' "$output_dir"
