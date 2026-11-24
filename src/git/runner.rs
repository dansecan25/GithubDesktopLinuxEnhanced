use std::fmt;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::constants::git::GIT_BINARY;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub message: String,
}

impl GitError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GitError {}

pub type GitResult<T> = Result<T, GitError>;

pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Every git call in the app goes through here.
///
/// Prompts are disabled because there is no terminal to answer them: credentials come from the
/// user's credential helper or SSH agent, exactly like `git` in a script.
pub fn run_raw(dir: &Path, args: &[&str], stdin: Option<&str>) -> GitResult<GitOutput> {
    let mut command = Command::new(GIT_BINARY);
    command
        .current_dir(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        // Without this, ssh would wait forever for a passphrase or host-key answer nobody can type.
        command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }

    let mut child =
        command.spawn().map_err(|error| GitError::new(format!("Could not start git: {error}")))?;
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(input.as_bytes())
            .map_err(|error| GitError::new(format!("Could not write to git: {error}")))?;
    }
    let output =
        child.wait_with_output().map_err(|error| GitError::new(format!("git did not finish: {error}")))?;

    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Runs git and returns stdout, or stderr as the error when git exits with a non-zero code.
pub fn run(dir: &Path, args: &[&str]) -> GitResult<String> {
    run_with_input(dir, args, None)
}

pub fn run_with_input(dir: &Path, args: &[&str], stdin: Option<&str>) -> GitResult<String> {
    let output = run_raw(dir, args, stdin)?;
    if output.exit_code == 0 { Ok(output.stdout) } else { Err(error_from_output(&output)) }
}

fn error_from_output(output: &GitOutput) -> GitError {
    let stderr = output.stderr.trim();
    let message = if stderr.is_empty() { output.stdout.trim() } else { stderr };
    if message.is_empty() {
        GitError::new(format!("git exited with code {}", output.exit_code))
    } else {
        GitError::new(message)
    }
}
