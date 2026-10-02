# Corral core

Rust agent runtime shipped with Saddle. This package builds independently and has no
Saddle, plugin, task, routing or telemetry dependencies. It does not execute the old
Corral CLI or require a Python interpreter. Its public executable remains `corral`.

```sh
cargo build --manifest-path crates/corral-core/Cargo.toml --bin corral
cargo test --manifest-path crates/corral-core/Cargo.toml --all-targets
```

The CLI, pen protocol v1, event format v1 and cursor v2 follow the original Corral
baseline `6923da1`. The original repository is reference material and remains
unchanged. Claude/Codex hooks and detached pen/after workers run the version-bound
Rust executable; pi/omp load their own TypeScript extensions.

Pens outlive clients and TUI instances. Use `corral attach NAME` independently;
Ctrl-] detaches, while explicit `corral stop NAME` stops the agent. New binaries
must live in immutable version directories: do not overwrite or remove a binary
still referenced by a running pen or hook. `scripts/package.sh NEW_DIRECTORY`
builds the host, runtime and official process plugins together without installing
anything. Changing global links/configuration is a separate deployment operation.

Tests use isolated HOME/CORRAL_HOME and copied, fixed runtime binaries. No real
coding agent or user state is required. Optional ignored interoperability tests
accept `CORRAL_COMPAT_BIN` pointing to an unchanged external v1 baseline; this is
only a comparison tool, never a build or runtime dependency.

`install-skills` retains consent and ownership checks. Existing symlinked skills
are reported as foreign and left untouched, so a source-repository link cannot
make installation write into that repository. Deployment must handle these links
explicitly. The command never installs skills on ordinary startup.
