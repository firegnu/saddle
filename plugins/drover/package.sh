#!/bin/sh
# Produce a local plugin directory; no Saddle registration or global installation.
set -eu
if [ "$#" -gt 1 ]; then
    echo "Usage: $0 [output-directory]" >&2
    exit 2
fi
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir=${1:-"$project_dir/dist/drover-plugin"}
build_dir=${CARGO_TARGET_DIR:-"$project_dir/target"}
host=$(rustc -vV | sed -n 's/^host: //p')

cargo install --path "$project_dir" --bin saddle-drover --locked \
    --root "$output_dir" --target-dir "$build_dir" --target "$host" \
    --no-track --force
cp "$project_dir/plugin.toml" "$output_dir/plugin.toml"
output_dir=$(CDPATH= cd -- "$output_dir" && pwd)
printf '\nPlugin directory: %s\nSettings → Plugins → Add local → Read manifest → Add disabled → Enable; then close Settings, click Plugins, select Drover and press Enter.\n' "$output_dir"
