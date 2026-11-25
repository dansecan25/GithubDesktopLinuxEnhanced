//! Limits that keep a stuck or runaway app from freezing the whole computer. See
//! `services/watchdog.rs` and `services/memory_cap.rs`.

use std::time::Duration;

/// How often the watchdog checks the window and the app's memory.
pub const WATCHDOG_INTERVAL: Duration = Duration::from_secs(1);

/// A frame that takes longer than this counts as a freeze, and a report is written.
pub const FREEZE_REPORT_AFTER: Duration = Duration::from_secs(5);

/// Past this much memory the app writes a report and closes, before it slows down the computer.
pub const SOFT_MEMORY_LIMIT_BYTES: u64 = 1536 * 1024 * 1024;

/// The system's limit on the app and the git commands it runs, in systemd's format. Above the
/// soft limit, so normally the app closes itself first and writes a report.
pub const HARD_MEMORY_LIMIT: &str = "2G";
/// Swap the app may use. Kept small: heavy swapping is what freezes the desktop.
pub const HARD_SWAP_LIMIT: &str = "256M";

/// Set to `applied` once the app runs inside its memory-limited scope. Set it to anything else
/// (for example `off`) to start without the limit.
pub const MEMORY_CAP_ENV: &str = "GDE_MEMORY_CAP";
pub const MEMORY_CAP_APPLIED: &str = "applied";

/// Starts programs in their own systemd scope.
pub const SYSTEMD_RUN: &str = "systemd-run";

/// Reports are kept in `<state folder>/github-desktop-enhanced/reports`. Only the newest are kept.
pub const REPORTS_FOLDER: &str = "reports";
pub const MAX_REPORTS: usize = 20;
/// Written when the app closes itself, and shown on the next start.
pub const LAST_EXIT_FILE: &str = "last-exit.txt";
/// Exit code when the app closes itself for using too much memory.
pub const MEMORY_LIMIT_EXIT_CODE: i32 = 3;
