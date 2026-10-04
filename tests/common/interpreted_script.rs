use std::{fs, os::unix::fs::symlink, path::Path};

/// Opt-in for tests whose short budgets must measure the scripted behavior.
/// New script executables can stall before their first instruction on macOS.
/// A symlink reuses one Cargo-built native launcher; the script is only read as data.
/// The launcher execs the interpreter, preserving PID, process group and pipes.
pub fn script(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(dir.join(format!("{name}.script")), text).unwrap();
    symlink(env!("CARGO_BIN_EXE_saddle-test-script"), &path).unwrap();
    path.to_str().unwrap().into()
}
