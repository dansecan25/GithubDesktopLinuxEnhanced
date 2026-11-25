//! Opening a repository or file in the user's code editor.

use std::path::{Path, PathBuf};

use super::memory_cap::outside_cap;
use super::system::spawn_detached;
use crate::constants::editors::KNOWN_EDITORS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    pub name: String,
    pub command: String,
}

/// Editors from [`KNOWN_EDITORS`] that are installed (found on `PATH`).
pub fn detect() -> Vec<Editor> {
    let mut found: Vec<Editor> = Vec::new();
    for (name, command) in KNOWN_EDITORS {
        if find_in_path(command).is_some() && !found.iter().any(|editor| editor.name == *name) {
            found.push(Editor { name: (*name).to_owned(), command: (*command).to_owned() });
        }
    }
    found
}

pub fn find_in_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|dir| dir.join(program)).find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata().is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// Splits a command line such as `code --new-window` or `"/opt/My Editor/bin/editor" -n` into the
/// program and its arguments.
pub fn split_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in command.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Opens `path` with `command` (the chosen editor), without waiting for it.
pub fn open(command: &str, path: &Path) -> Result<(), String> {
    let parts = split_command(command);
    let (program, arguments) = parts.split_first().ok_or("Choose an editor in Preferences first")?;
    let mut process = outside_cap(program);
    process.args(arguments).arg(path);
    spawn_detached(process).map_err(|error| format!("Could not start {program}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::split_command;

    #[test]
    fn splits_quoted_commands() {
        assert_eq!(split_command("code --new-window"), ["code", "--new-window"]);
        assert_eq!(split_command("\"/opt/My Editor/bin/ed\"  -n"), ["/opt/My Editor/bin/ed", "-n"]);
        assert!(split_command("   ").is_empty());
    }
}
