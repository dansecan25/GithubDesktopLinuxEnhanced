//! Runs the app inside a systemd scope with a memory limit. If the app, or a git command it
//! started, runs away with memory, the system stops it instead of the whole desktop freezing.

use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::constants::safety::{
    HARD_MEMORY_LIMIT, HARD_SWAP_LIMIT, MEMORY_CAP_APPLIED, MEMORY_CAP_ENV, SYSTEMD_RUN,
};

/// Restarts this process inside a memory-limited scope. Returns only when that isn't possible (no
/// systemd user session, already limited, or turned off with `GDE_MEMORY_CAP=off`); the app then
/// runs without the limit.
pub fn relaunch_capped() {
    if std::env::var_os(MEMORY_CAP_ENV).is_some() || !user_session_available() {
        return;
    }
    let Ok(executable) = std::env::current_exe() else { return };
    let error = Command::new(SYSTEMD_RUN)
        .args(["--user", "--scope", "--quiet", "--collect"])
        .arg(format!("--property=MemoryMax={HARD_MEMORY_LIMIT}"))
        .arg(format!("--property=MemorySwapMax={HARD_SWAP_LIMIT}"))
        .arg("--")
        .arg(executable)
        .args(std::env::args_os().skip(1))
        .env(MEMORY_CAP_ENV, MEMORY_CAP_APPLIED)
        .exec();
    // `exec` only returns when systemd-run couldn't be started.
    eprintln!("Running without a memory limit: {error}");
}

pub fn is_capped() -> bool {
    std::env::var_os(MEMORY_CAP_ENV).is_some_and(|value| value == MEMORY_CAP_APPLIED)
}

/// systemd-run needs the user's session bus, or it fails and the app wouldn't start.
fn user_session_available() -> bool {
    let bus = std::env::var_os("XDG_RUNTIME_DIR").map(|dir| Path::new(&dir).join("bus"));
    bus.is_some_and(|bus| bus.exists())
}

/// A command for another program (an editor, the file manager). When the app is limited, the
/// program gets its own scope, so it neither counts against the app's limit nor gets stopped with it.
pub fn outside_cap(program: impl AsRef<OsStr>) -> Command {
    if is_capped() {
        let mut command = Command::new(SYSTEMD_RUN);
        command.args(["--user", "--scope", "--quiet", "--collect", "--"]).arg(program);
        // Started from inside the limit, it would otherwise inherit the variable.
        command.env_remove(MEMORY_CAP_ENV);
        command
    } else {
        Command::new(program)
    }
}
