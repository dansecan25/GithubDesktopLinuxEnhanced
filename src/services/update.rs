//! Checks this app's GitHub releases for a newer version, downloads the package for this system
//! in the background and installs it with the package's own `install.sh`. Same flow as the owner's
//! other desktop apps: check on start and every 12 hours, then **Install** and **Restart now** or
//! **Later**.
//!
//! Only the copy that `install.sh` put in place updates itself. A build run from a source checkout
//! is left alone.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use eframe::egui::Context;

use crate::constants::app::{APP_ID, VERSION};
use crate::constants::github::{APP_REPOSITORY, DOWNLOAD_TIMEOUT, MAX_PACKAGE_BYTES, UPDATE_CHECK_INTERVAL};
use crate::github;
use crate::utils::version::is_newer;

/// A downloaded package, ready to install.
#[derive(Clone, Debug)]
pub struct Update {
    pub version: String,
    installer: PathBuf,
}

#[derive(Clone, Debug)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    /// Checking or downloading failed, for example while offline. Shown only in Preferences, since
    /// the next check usually works.
    CheckFailed(String),
    Downloading(String),
    Ready(Update),
    Installing(String),
    Installed(String),
    InstallFailed(String),
}

/// The package name `release.sh` gives this system's build.
fn package_name(version: &str) -> Option<String> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        _ => return None,
    };
    let arch = match (os, std::env::consts::ARCH) {
        ("darwin", "aarch64") => "arm64",
        (_, arch) => arch,
    };
    Some(format!("{APP_ID}-{version}-{os}-{arch}"))
}

/// Where `install.sh` puts the program.
fn installed_executable() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".local/bin").join(APP_ID))
}

/// True if this is the copy `install.sh` put in place.
pub fn can_update() -> bool {
    let canonical = |path: PathBuf| fs::canonicalize(path).ok();
    let running = std::env::current_exe().ok().and_then(canonical);
    let installed = installed_executable().and_then(canonical);
    running.is_some() && running == installed
}

fn download(url: &str, to: &Path) -> Result<(), String> {
    let error = |e: &dyn std::fmt::Display| format!("Could not download the update: {e}");
    let mut response = github::agent(DOWNLOAD_TIMEOUT).get(url).call().map_err(|e| error(&e))?;
    if !response.status().is_success() {
        return Err(error(&response.status()));
    }
    let mut reader = response.body_mut().with_config().limit(MAX_PACKAGE_BYTES).reader();
    let mut file = fs::File::create(to).map_err(|e| error(&e))?;
    io::copy(&mut reader, &mut file).map_err(|e| error(&e))?;
    Ok(())
}

/// Downloads and unpacks the latest release if it is newer than `current`.
fn fetch_update(current: &str, report: &dyn Fn(Status)) -> Result<Option<Update>, String> {
    let release = github::latest_release(APP_REPOSITORY)?;
    let version = release.tag_name.trim_start_matches('v').to_owned();
    if !is_newer(&version, current) {
        return Ok(None);
    }
    let name = package_name(&version).ok_or("Updates aren't available for this system")?;
    let file_name = format!("{name}.tar.gz");
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == file_name)
        .ok_or_else(|| format!("Version {version} has no package for this system yet"))?;

    report(Status::Downloading(version.clone()));
    let dir = dirs::cache_dir().ok_or("No cache folder on this system")?.join(APP_ID).join("update");
    // Only one update is kept, so an older download is removed.
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let archive = dir.join(&file_name);
    download(&asset.browser_download_url, &archive)?;

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Could not unpack the update: {e}"))?;
    let installer = dir.join(&name).join("install.sh");
    if !status.success() || !installer.is_file() {
        return Err("The downloaded update is damaged. It will be downloaded again later.".into());
    }
    let _ = fs::remove_file(&archive);
    Ok(Some(Update { version, installer }))
}

fn install(update: &Update) -> Result<(), String> {
    let output = Command::new("bash")
        .arg(&update.installer)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Could not run the installer: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let reason = stderr.lines().last().unwrap_or("unknown error").trim();
    Err(format!("The update couldn't be installed: {reason}"))
}

pub struct Updater {
    pub status: Status,
    updates: Receiver<Status>,
    sender: Sender<Status>,
    last_check: Option<Instant>,
    /// Hides the update notice until the next start.
    pub dismissed: bool,
}

impl Default for Updater {
    fn default() -> Self {
        let (sender, updates) = channel();
        Self { status: Status::Idle, updates, sender, last_check: None, dismissed: false }
    }
}

impl Updater {
    fn busy(&self) -> bool {
        matches!(self.status, Status::Checking | Status::Downloading(_) | Status::Installing(_))
    }

    /// Runs `work` on a thread, which reports progress through the channel.
    fn run(&mut self, ctx: &Context, first: Status, work: impl FnOnce(&dyn Fn(Status)) + Send + 'static) {
        self.status = first;
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let report = |status: Status| {
                let _ = sender.send(status);
                ctx.request_repaint();
            };
            work(&report);
        });
    }

    pub fn check_now(&mut self, ctx: &Context) {
        if self.busy() || !can_update() {
            return;
        }
        self.last_check = Some(Instant::now());
        self.run(ctx, Status::Checking, |report| {
            report(match fetch_update(VERSION, report) {
                Ok(Some(update)) => Status::Ready(update),
                Ok(None) => Status::UpToDate,
                Err(error) => Status::CheckFailed(error),
            });
        });
    }

    pub fn install(&mut self, ctx: &Context) {
        let Status::Ready(update) = self.status.clone() else { return };
        let installing = Status::Installing(update.version.clone());
        self.run(ctx, installing, move |report| {
            report(match install(&update) {
                Ok(()) => Status::Installed(update.version),
                Err(error) => Status::InstallFailed(error),
            });
        });
    }

    /// Takes progress reports, and starts a check when one is due.
    pub fn poll(&mut self, ctx: &Context, automatic: bool) {
        while let Ok(status) = self.updates.try_recv() {
            self.status = status;
        }
        let waiting = matches!(self.status, Status::Idle | Status::UpToDate | Status::CheckFailed(_));
        if automatic && waiting {
            if self.last_check.is_none_or(|at| at.elapsed() >= UPDATE_CHECK_INTERVAL) {
                self.check_now(ctx);
            } else {
                ctx.request_repaint_after(UPDATE_CHECK_INTERVAL);
            }
        }
    }
}

/// Starts the newly installed version. The caller then closes this one.
pub fn restart() -> Result<(), String> {
    let executable = installed_executable().ok_or("No home folder")?;
    super::system::spawn_detached(Command::new(executable))
        .map_err(|e| format!("Could not start the new version: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names_match_release_sh() {
        let name = package_name("1.2.3").unwrap();
        assert!(name.starts_with("github-desktop-enhanced-1.2.3-"));
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(name, "github-desktop-enhanced-1.2.3-linux-x86_64");
        }
    }
}
