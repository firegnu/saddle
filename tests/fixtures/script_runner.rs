//! Execute a fixture's sidecar through its interpreter, without a fresh script exec.
use std::{fs, os::unix::process::CommandExt, process::Command};

fn main() {
    // Keep the symlink path: each fixture owns its script and data directory.
    let mut args = std::env::args_os();
    let mut script = args.next().unwrap();
    script.push(".script");
    let text = fs::read_to_string(&script).unwrap();
    let mut interpreter = text
        .lines()
        .next()
        .unwrap()
        .strip_prefix("#!")
        .unwrap()
        .split_whitespace();
    let error = Command::new(interpreter.next().unwrap())
        .args(interpreter)
        .arg(script)
        .args(args)
        .exec();
    panic!("fixture interpreter exec failed: {error}");
}
