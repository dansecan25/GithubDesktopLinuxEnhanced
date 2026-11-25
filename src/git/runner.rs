use std::fmt;
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::control::{self, OperationControl};
use crate::constants::app::{ASKPASS_SOCKET_ENV, ASKPASS_TOKEN_ENV};
use crate::constants::git::{
    GIT_BINARY, GIT_POLL_MAX, GIT_STOP_GRACE, GIT_TIMEOUT, MAX_GIT_ERROR_BYTES, MAX_GIT_OUTPUT_BYTES,
    UNTIMED_GIT_COMMANDS,
};
use crate::constants::text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub message: String,
    /// The user stopped the operation, so there's nothing to report.
    pub cancelled: bool,
}

impl GitError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), cancelled: false }
    }

    pub fn cancelled() -> Self {
        Self { message: text::CANCELLED.to_owned(), cancelled: true }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GitError {}

pub type GitResult<T> = Result<T, GitError>;

/// How git reaches the running app for credentials. Set once at startup by
/// `services::askpass::AskpassServer::start`; without it (tests, CLI use) git never prompts.
#[derive(Debug, Clone)]
pub struct AskpassEnvironment {
    pub executable: PathBuf,
    pub socket: PathBuf,
    pub token: String,
}

static ASKPASS: OnceLock<AskpassEnvironment> = OnceLock::new();

pub fn configure_askpass(environment: AskpassEnvironment) {
    let _ = ASKPASS.set(environment);
}

pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    /// stdout went over the size limit, so git was stopped and `stdout` holds only the start.
    pub truncated: bool,
}

const SIGKILL: i32 = 9;

/// Git commands running right now, for the watchdog's freeze reports.
static RUNNING: Mutex<Vec<(u64, String, Instant)>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Each running git command (credentials removed) and how long it has been running.
pub fn running_commands() -> Vec<(String, Duration)> {
    let running = RUNNING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    running.iter().map(|(_, command, started)| (command.clone(), started.elapsed())).collect()
}

/// Lists a command in [`RUNNING`] until dropped.
struct Registration(u64);

impl Registration {
    fn new(args: &[&str]) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let command = describe(args);
        RUNNING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push((id, command, Instant::now()));
        Self(id)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        RUNNING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).retain(|(id, ..)| *id != self.0);
    }
}

/// `git <args>`, with any `user:password@` in a URL removed.
fn describe(args: &[&str]) -> String {
    let words = args.iter().map(|arg| match arg.split_once("://") {
        Some((scheme, rest)) if rest.split('/').next().is_some_and(|host| host.contains('@')) => {
            let after_at = rest.split_once('@').map_or(rest, |(_, after)| after);
            format!("{scheme}://***@{after_at}")
        }
        _ => (*arg).to_owned(),
    });
    std::iter::once(GIT_BINARY.to_owned()).chain(words).collect::<Vec<_>>().join(" ")
}

/// Every git call in the app goes through here.
///
/// Terminal prompts are disabled because there is no terminal. Credentials come from the user's
/// credential helper or SSH agent, and otherwise from the askpass bridge, which asks in the app.
///
/// Output past `MAX_GIT_OUTPUT_BYTES` stops git, and local commands are stopped after
/// `GIT_TIMEOUT`, so one bad command can't use up the computer's memory or hang forever.
pub fn run_raw(dir: &Path, args: &[&str], stdin: Option<&str>) -> GitResult<GitOutput> {
    run_limited(dir, args, stdin, MAX_GIT_OUTPUT_BYTES)
}

fn run_limited(dir: &Path, args: &[&str], stdin: Option<&str>, max_stdout: usize) -> GitResult<GitOutput> {
    let (stdout, output) = run_limited_bytes(dir, args, stdin, max_stdout)?;
    Ok(GitOutput { stdout: String::from_utf8_lossy(&stdout).into_owned(), ..output })
}

/// Raw stdout (for binary content such as images), with the rest of the output in a [`GitOutput`]
/// whose `stdout` is empty.
fn run_limited_bytes(
    dir: &Path,
    args: &[&str],
    stdin: Option<&str>,
    max_stdout: usize,
) -> GitResult<(Vec<u8>, GitOutput)> {
    let mut command = Command::new(GIT_BINARY);
    command
        .current_dir(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(askpass) = ASKPASS.get() {
        command
            .env("GIT_ASKPASS", &askpass.executable)
            .env("SSH_ASKPASS", &askpass.executable)
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env(ASKPASS_SOCKET_ENV, &askpass.socket)
            .env(ASKPASS_TOKEN_ENV, &askpass.token);
    } else if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        // Without a way to ask, ssh would wait forever for an answer nobody can type.
        command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }

    let control = control::current();
    if control.as_ref().is_some_and(|control| control.is_cancelled()) {
        return Err(GitError::cancelled());
    }
    let mut child =
        command.spawn().map_err(|error| GitError::new(format!("Could not start git: {error}")))?;
    let _running = Registration::new(args);
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(input.as_bytes())
            .map_err(|error| GitError::new(format!("Could not write to git: {error}")))?;
    }
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return Err(GitError::new("Could not read git's output"));
    };
    let stdout = std::thread::spawn(move || read_capped(stdout, max_stdout));
    let stderr_control = control.clone();
    let stderr = std::thread::spawn(move || match stderr_control {
        Some(control) => control::read_stderr(stderr, MAX_GIT_ERROR_BYTES, &control),
        None => read_capped(stderr, MAX_GIT_ERROR_BYTES),
    });

    let timeout = args.first().filter(|name| !UNTIMED_GIT_COMMANDS.contains(name)).map(|_| GIT_TIMEOUT);
    let ending = wait(&mut child, timeout, control.as_deref())
        .map_err(|error| GitError::new(format!("git did not finish: {error}")))?;
    // When git was stopped the readers aren't joined: a process it started may still hold the pipes.
    let status = match ending {
        Ending::Exited(status) => status,
        Ending::Cancelled => return Err(GitError::cancelled()),
        Ending::TimedOut => {
            return Err(GitError::new(format!(
                "`{}` took more than {} minutes, so it was stopped.",
                describe(args),
                GIT_TIMEOUT.as_secs() / 60
            )));
        }
    };
    // SIGKILL that wasn't the timeout: the memory limit (see `services/memory_cap.rs`).
    if status.signal() == Some(SIGKILL) {
        return Err(GitError::new(text::git_out_of_memory(&describe(args))));
    }
    let (stdout, truncated) = stdout.join().unwrap_or_default();
    let (stderr, _) = stderr.join().unwrap_or_default();

    let output = GitOutput {
        stdout: String::new(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        exit_code: status.code().unwrap_or(-1),
        truncated,
    };
    Ok((stdout, output))
}

/// Reads up to `max` bytes. Stopping early closes the pipe, which ends git with SIGPIPE.
fn read_capped(pipe: impl Read, max: usize) -> (Vec<u8>, bool) {
    let mut bytes = Vec::new();
    let _ = pipe.take(max as u64 + 1).read_to_end(&mut bytes);
    let truncated = bytes.len() > max;
    bytes.truncate(max);
    (bytes, truncated)
}

enum Ending {
    Exited(ExitStatus),
    TimedOut,
    Cancelled,
}

/// Waits for git, stopping it after `timeout` or when the operation is cancelled.
fn wait(
    child: &mut Child,
    timeout: Option<Duration>,
    control: Option<&OperationControl>,
) -> std::io::Result<Ending> {
    if timeout.is_none() && control.is_none() {
        return child.wait().map(Ending::Exited);
    }
    let deadline = timeout.map(|timeout| Instant::now() + timeout);
    let mut pause = Duration::from_millis(1);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Ending::Exited(status));
        }
        if control.is_some_and(OperationControl::is_cancelled) {
            stop(child);
            return Ok(Ending::Cancelled);
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Ending::TimedOut);
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(GIT_POLL_MAX);
    }
}

/// Asks git to stop (SIGTERM), so a clone removes its half-made folder, and kills it if it hasn't
/// stopped after `GIT_STOP_GRACE`.
fn stop(child: &mut Child) {
    let _ = Command::new("kill").arg("-TERM").arg(child.id().to_string()).status();
    let deadline = Instant::now() + GIT_STOP_GRACE;
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(GIT_POLL_MAX);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Runs git and returns stdout, or stderr as the error when git exits with a non-zero code.
pub fn run(dir: &Path, args: &[&str]) -> GitResult<String> {
    run_with_input(dir, args, None)
}

pub fn run_with_input(dir: &Path, args: &[&str], stdin: Option<&str>) -> GitResult<String> {
    let output = run_raw(dir, args, stdin)?;
    if output.truncated {
        return Err(GitError::new(format!(
            "`{}` printed more than {} MB, so it was stopped.",
            describe(args),
            MAX_GIT_OUTPUT_BYTES / (1024 * 1024)
        )));
    }
    if output.exit_code == 0 { Ok(output.stdout) } else { Err(error_from_output(&output)) }
}

/// Runs git and returns its raw stdout, for binary content. Output over `max_bytes` is an error.
pub fn run_binary(dir: &Path, args: &[&str], max_bytes: usize) -> GitResult<Vec<u8>> {
    let (stdout, output) = run_limited_bytes(dir, args, None, max_bytes)?;
    if output.truncated {
        return Err(GitError::new(format!("`{}` printed more than {max_bytes} bytes", describe(args))));
    }
    if output.exit_code == 0 { Ok(stdout) } else { Err(error_from_output(&output)) }
}

/// Like [`run`], but keeps only the first `max_bytes` of the output instead of failing. For diffs,
/// which are cut for display anyway.
pub fn run_capped(dir: &Path, args: &[&str], max_bytes: usize) -> GitResult<String> {
    let output = run_limited(dir, args, None, max_bytes)?;
    if output.exit_code == 0 || output.truncated {
        Ok(output.stdout)
    } else {
        Err(error_from_output(&output))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptions_hide_url_credentials() {
        assert_eq!(
            describe(&["clone", "--", "https://user:secret@github.com/o/r.git"]),
            "git clone -- https://***@github.com/o/r.git"
        );
        assert_eq!(describe(&["clone", "https://github.com/o/r"]), "git clone https://github.com/o/r");
        assert_eq!(describe(&["status"]), "git status");
    }

    #[test]
    fn capped_reads_stop_at_the_limit() {
        assert_eq!(read_capped(&b"abcdef"[..], 4), (b"abcd".to_vec(), true));
        assert_eq!(read_capped(&b"abc"[..], 4), (b"abc".to_vec(), false));
    }
}
