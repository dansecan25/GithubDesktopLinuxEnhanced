use std::ffi::OsStr;
use std::process::{Command, Stdio};

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(not(target_os = "macos"))]
const OPENER: &str = "xdg-open";

/// Starts a program without waiting for it. A thread waits in the background so the finished
/// process doesn't linger as a zombie.
pub fn spawn_detached(mut command: Command) -> std::io::Result<()> {
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Opens a folder in the file manager, a file in its default app, or a URL in the browser.
pub fn open_with_system(target: impl AsRef<OsStr>) -> std::io::Result<()> {
    let mut command = super::memory_cap::outside_cap(OPENER);
    command.arg(target);
    spawn_detached(command)
}
