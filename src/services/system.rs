use std::path::Path;
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

/// Opens a folder in the file manager, or a file in its default app.
pub fn open_with_system(path: &Path) -> std::io::Result<()> {
    let mut command = Command::new(OPENER);
    command.arg(path);
    spawn_detached(command)
}
