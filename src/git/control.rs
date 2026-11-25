//! Following and stopping a long git operation (clone, fetch, pull, push) from the UI.
//!
//! The app gives each operation an [`OperationControl`] and runs it inside [`with_control`]. Every
//! git command the operation starts on that thread then reports its progress to the control, and
//! is stopped when the control is cancelled.

use std::cell::RefCell;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// What git last reported, for example "Receiving objects" at 45%.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitProgress {
    pub phase: String,
    pub percent: Option<u8>,
}

#[derive(Debug, Default)]
pub struct OperationControl {
    cancelled: AtomicBool,
    progress: Mutex<Option<GitProgress>>,
}

impl OperationControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn progress(&self) -> Option<GitProgress> {
        self.progress.lock().ok()?.clone()
    }

    fn set_progress(&self, progress: GitProgress) {
        if let Ok(mut current) = self.progress.lock() {
            *current = Some(progress);
        }
    }
}

thread_local! {
    static CONTROL: RefCell<Option<Arc<OperationControl>>> = const { RefCell::new(None) };
}

/// Runs `job` with `control` attached to every git command it starts on this thread.
pub fn with_control<T>(control: Arc<OperationControl>, job: impl FnOnce() -> T) -> T {
    CONTROL.with(|current| *current.borrow_mut() = Some(control));
    let result = job();
    CONTROL.with(|current| *current.borrow_mut() = None);
    result
}

pub(super) fn current() -> Option<Arc<OperationControl>> {
    CONTROL.with(|current| current.borrow().clone())
}

/// Reads git's stderr, reporting progress lines to `control`. Progress lines end in `\r` and are
/// left out of the returned text, so an error message doesn't fill up with percentages.
pub(super) fn read_stderr(mut pipe: impl Read, max: usize, control: &OperationControl) -> (Vec<u8>, bool) {
    // A line this long is not a progress line; it's kept only up to here.
    const MAX_LINE: usize = 64 * 1024;
    let (mut kept, mut line, mut truncated) = (Vec::new(), Vec::new(), false);
    let mut buffer = [0; 4096];
    while let Ok(count) = pipe.read(&mut buffer) {
        if count == 0 {
            break;
        }
        for &byte in &buffer[..count] {
            if byte != b'\r' && byte != b'\n' {
                if line.len() < MAX_LINE {
                    line.push(byte);
                }
                continue;
            }
            if let Some(progress) = parse_progress(&String::from_utf8_lossy(&line)) {
                control.set_progress(progress);
            }
            if byte == b'\n' && !line.is_empty() {
                if kept.len() + line.len() < max {
                    kept.extend_from_slice(&line);
                    kept.push(b'\n');
                } else {
                    truncated = true;
                }
            }
            line.clear();
        }
    }
    kept.extend_from_slice(&line);
    (kept, truncated)
}

/// `Receiving objects:  45% (450/1000), 1.2 MiB | 2 MiB/s` → ("Receiving objects", 45).
pub fn parse_progress(line: &str) -> Option<GitProgress> {
    let line = line.trim();
    let line = line.strip_prefix("remote:").map_or(line, str::trim);
    let (phase, rest) = line.split_once(':')?;
    let rest = rest.trim();
    let percent = rest.split_once('%').and_then(|(number, _)| number.trim().parse::<u8>().ok());
    // "Counting objects: 12, done." has no percentage but is still progress; other lines aren't.
    let looks_like_progress = percent.is_some() || rest.starts_with(|c: char| c.is_ascii_digit());
    let phase_ok = !phase.trim().is_empty() && phase.len() < 40;
    (looks_like_progress && phase_ok)
        .then(|| GitProgress { phase: phase.trim().to_owned(), percent: percent.map(|p| p.min(100)) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_git_progress_lines() {
        let progress = parse_progress("Receiving objects:  45% (450/1000), 1.2 MiB | 2 MiB/s").unwrap();
        assert_eq!(progress, GitProgress { phase: "Receiving objects".into(), percent: Some(45) });
        let remote = parse_progress("remote: Compressing objects: 100% (8/8), done.").unwrap();
        assert_eq!(remote.phase, "Compressing objects");
        assert_eq!(remote.percent, Some(100));
        assert_eq!(parse_progress("remote: Enumerating objects: 12, done.").unwrap().percent, None);
        assert!(parse_progress("Cloning into 'repo'...").is_none());
        assert!(parse_progress("fatal: repository 'x' does not exist").is_none());
        assert!(parse_progress("To https://github.com/o/r.git").is_none());
    }

    #[test]
    fn progress_lines_are_left_out_of_stderr() {
        let control = OperationControl::default();
        let stderr = b"Receiving objects:  50% (1/2)\rReceiving objects: 100% (2/2), done.\nerror: oops\n";
        let (kept, _) = read_stderr(&stderr[..], 1024, &control);
        assert_eq!(String::from_utf8_lossy(&kept), "Receiving objects: 100% (2/2), done.\nerror: oops\n");
        assert_eq!(control.progress().unwrap().percent, Some(100));
    }
}
