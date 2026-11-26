//! Failsafes for a stuck or runaway app.
//!
//! A background thread checks once a second that the window isn't stuck in a frame and that the
//! app's memory stays under `SOFT_MEMORY_LIMIT_BYTES`. A stuck frame writes a report, with what the
//! app was doing and which git commands were running. So does a focused window that stops being
//! drawn at all (the graphics stack stuck, outside any frame). Going over the memory limit writes a report
//! and closes the app before it can slow down the computer. Only `main` starts it; without it
//! (tests) every call here does nothing.

use std::fmt::Debug;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;

use crate::constants::app::{APP_NAME, CONFIG_DIR_NAME, VERSION};
use crate::constants::safety::{
    FREEZE_REPORT_AFTER, LAST_EXIT_FILE, MAX_REPORTS, MEMORY_LIMIT_EXIT_CODE, REPORTS_FOLDER,
    SOFT_MEMORY_LIMIT_BYTES, STALL_REPORT_AFTER, WATCHDOG_INTERVAL,
};
use crate::constants::text;
use crate::git::runner;
use crate::services::memory_cap;

const MIB: u64 = 1024 * 1024;

#[derive(Default)]
struct Frame {
    /// When the frame being drawn started. `None` between frames.
    started: Option<Instant>,
    /// What the app is doing in this frame.
    activity: String,
    /// Whether this frame got stuck, and the report written for it.
    stuck: bool,
    report: Option<PathBuf>,
    /// When the last frame started, and whether the window had focus then.
    last_started: Option<Instant>,
    focused: bool,
    /// The report written for a window that stopped being drawn, until it's drawn again.
    stall_report: Option<PathBuf>,
}

static FRAME: OnceLock<Mutex<Frame>> = OnceLock::new();
/// Lets the watchdog ask the window to redraw.
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();
/// The GPU and driver drawing the window, for reports.
static GRAPHICS: OnceLock<String> = OnceLock::new();

fn frame() -> Option<MutexGuard<'static, Frame>> {
    FRAME.get().map(|frame| frame.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
}

pub fn start() {
    if FRAME.set(Mutex::new(Frame::default())).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new().name("watchdog".into()).spawn(|| {
        loop {
            std::thread::sleep(WATCHDOG_INTERVAL);
            check_memory();
            check_frame();
            check_stall();
        }
    });
    if let Err(error) = spawned {
        eprintln!("The watchdog could not start: {error}");
    }
}

/// Gives the watchdog the window's context, so it can ask for redraws.
pub fn attach(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}

/// Records which GPU draws the window, for reports.
pub fn set_graphics(description: String) {
    let _ = GRAPHICS.set(description);
}

/// Call every frame with whether the window has focus.
pub fn set_focused(focused: bool) {
    if let Some(mut frame) = frame() {
        frame.focused = focused;
    }
}

/// Call at the start of every frame.
pub fn frame_started() {
    if let Some(mut frame) = frame() {
        let now = Instant::now();
        if let Some(report) = frame.stall_report.take() {
            let seconds = frame.last_started.map_or(0, |last| now.duration_since(last).as_secs());
            append(&report, &format!("\nThe window was drawn again after {seconds} s.\n"));
        }
        frame.last_started = Some(now);
        frame.started = Some(now);
        frame.stuck = false;
        frame.report = None;
        "drawing the window".clone_into(&mut frame.activity);
    }
}

/// Records what the app is about to do, by the variant name of `value` (never its contents, which
/// can hold a token or a commit message).
pub fn doing(prefix: &str, value: &impl Debug) {
    if let Some(mut frame) = frame() {
        let debug = format!("{value:?}");
        let name: String = debug.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        frame.activity = format!("{prefix} {name}");
    }
}

/// Records what the app is about to do.
pub fn set_activity(activity: &str) {
    if let Some(mut frame) = frame() {
        activity.clone_into(&mut frame.activity);
    }
}

/// Call at the end of every frame. After a freeze, returns a message for the user.
pub fn frame_finished() -> Option<String> {
    let mut frame = frame()?;
    let started = frame.started.take()?;
    if !std::mem::take(&mut frame.stuck) {
        return None;
    }
    let report = frame.report.take()?;
    let seconds = started.elapsed().as_secs();
    append(&report, &format!("\nThe window responded again after {seconds} s.\n"));
    Some(text::window_was_stuck(seconds, &report.display().to_string()))
}

/// The message left by the last run if it closed itself for using too much memory.
pub fn take_last_exit_notice() -> Option<String> {
    let path = state_folder()?.join(LAST_EXIT_FILE);
    let notice = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(path);
    Some(notice)
}

fn check_frame() {
    let activity = {
        let Some(mut frame) = frame() else { return };
        let stuck = frame.started.is_some_and(|started| started.elapsed() >= FREEZE_REPORT_AFTER);
        if !stuck || frame.stuck {
            return;
        }
        frame.stuck = true;
        frame.activity.clone()
    };
    let reason =
        format!("The window stopped responding: a frame took over {} s.", FREEZE_REPORT_AFTER.as_secs());
    let report = write_report("freeze", &reason, &activity);
    if let Some(mut frame) = frame() {
        frame.report = report;
    }
}

/// A focused window is asked to redraw every second. If no frame follows for
/// `STALL_REPORT_AFTER`, the event loop or the graphics driver is stuck and a report is written.
fn check_stall() {
    let Some(ctx) = CONTEXT.get() else { return };
    let activity = {
        let Some(frame) = frame() else { return };
        if !frame.focused || frame.started.is_some() {
            return;
        }
        let stalled = frame.last_started.is_some_and(|last| last.elapsed() >= STALL_REPORT_AFTER);
        if !stalled || frame.stall_report.is_some() {
            drop(frame);
            ctx.request_repaint();
            return;
        }
        frame.activity.clone()
    };
    let reason = format!(
        "The window wasn't drawn for {} s while it had focus, although the app wasn't busy drawing. \
         The graphics driver or the window system stopped answering. If the screen was off or \
         locked at the time, this is expected.",
        STALL_REPORT_AFTER.as_secs()
    );
    let report = write_report("stall", &reason, &activity);
    if let Some(mut frame) = frame() {
        frame.stall_report = report;
    }
}

fn check_memory() {
    let Some(used) = resident_bytes() else { return };
    if used <= SOFT_MEMORY_LIMIT_BYTES {
        return;
    }
    let activity = frame().map(|frame| frame.activity.clone()).unwrap_or_default();
    let reason = format!(
        "The app used {} MB of memory, over its {} MB limit, so it closed itself.",
        used / MIB,
        SOFT_MEMORY_LIMIT_BYTES / MIB
    );
    let report = write_report("memory", &reason, &activity);
    if let (Some(folder), Some(report)) = (state_folder(), report) {
        let notice =
            text::closed_for_memory(used / MIB, SOFT_MEMORY_LIMIT_BYTES / MIB, &report.display().to_string());
        let _ = fs::write(folder.join(LAST_EXIT_FILE), notice);
    }
    eprintln!("{reason}");
    std::process::exit(MEMORY_LIMIT_EXIT_CODE);
}

/// `~/.local/state/github-desktop-enhanced`
fn state_folder() -> Option<PathBuf> {
    Some(dirs::state_dir().or_else(dirs::cache_dir)?.join(CONFIG_DIR_NAME))
}

fn write_report(kind: &str, reason: &str, activity: &str) -> Option<PathBuf> {
    let folder = state_folder()?.join(REPORTS_FOLDER);
    fs::create_dir_all(&folder).ok()?;
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_secs()).unwrap_or(0);
    let path = folder.join(format!("{seconds}-{kind}.log"));
    fs::write(&path, report_text(reason, activity, seconds)).ok()?;
    eprintln!("{reason} Report: {}", path.display());
    prune_reports(&folder);
    Some(path)
}

fn report_text(reason: &str, activity: &str, seconds: u64) -> String {
    let mut lines = vec![
        format!("{APP_NAME} {VERSION}"),
        reason.to_owned(),
        format!("Time: {seconds} (seconds since 1970, UTC)"),
        format!("Doing: {activity}"),
        format!("Graphics: {}", GRAPHICS.get().map_or("unknown", String::as_str)),
        format!("App memory: {} MB", resident_bytes().unwrap_or(0) / MIB),
        format!("Memory limit from the system: {}", if memory_cap::is_capped() { "yes" } else { "no" }),
    ];
    if let Some((available, total)) = system_memory() {
        lines.push(format!("System memory available: {} MB of {} MB", available / MIB, total / MIB));
    }
    let commands = runner::running_commands();
    if commands.is_empty() {
        lines.push("Git commands running: none".to_owned());
    } else {
        lines.push("Git commands running:".to_owned());
        lines
            .extend(commands.iter().map(|(command, time)| format!("  {command} (for {} s)", time.as_secs())));
    }
    lines.join("\n") + "\n"
}

fn append(path: &Path, text: &str) {
    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(path) {
        let _ = file.write_all(text.as_bytes());
    }
}

fn prune_reports(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    let mut reports: Vec<PathBuf> = entries.filter_map(|entry| Some(entry.ok()?.path())).collect();
    reports.sort();
    let excess = reports.len().saturating_sub(MAX_REPORTS);
    for old in &reports[..excess] {
        let _ = fs::remove_file(old);
    }
}

/// The app's resident memory, from `/proc/self/status`.
fn resident_bytes() -> Option<u64> {
    kibibytes(&fs::read_to_string("/proc/self/status").ok()?, "VmRSS:")
}

/// (available, total) system memory, from `/proc/meminfo`.
fn system_memory() -> Option<(u64, u64)> {
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    Some((kibibytes(&info, "MemAvailable:")?, kibibytes(&info, "MemTotal:")?))
}

/// Finds a `Name:   1234 kB` line and returns it in bytes.
fn kibibytes(text: &str, field: &str) -> Option<u64> {
    let line = text.lines().find(|line| line.starts_with(field))?;
    let value: u64 = line[field.len()..].split_whitespace().next()?.parse().ok()?;
    Some(value * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_proc_memory_fields() {
        let status = "Name:\tapp\nVmRSS:\t  2048 kB\nThreads: 4\n";
        assert_eq!(kibibytes(status, "VmRSS:"), Some(2048 * 1024));
        assert_eq!(kibibytes(status, "VmSwap:"), None);
    }

    #[test]
    fn this_process_has_resident_memory() {
        assert!(resident_bytes().is_some_and(|bytes| bytes > 0));
    }
}
