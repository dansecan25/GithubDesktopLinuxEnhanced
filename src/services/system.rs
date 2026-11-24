use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(not(target_os = "macos"))]
const OPENER: &str = "xdg-open";

/// Opens a folder in the desktop's file manager without waiting for it.
pub fn open_in_file_manager(path: &Path) -> std::io::Result<()> {
    Command::new(OPENER)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}
