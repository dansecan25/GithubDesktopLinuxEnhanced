//! Lets git and ssh ask the user for credentials while running in the background.
//!
//! Git can't prompt in a terminal here, so the app sets itself as `GIT_ASKPASS` and `SSH_ASKPASS`.
//! Git then starts this same executable with the prompt as its argument ([`run_helper`]). The
//! helper sends the prompt over a private Unix socket to the running app ([`AskpassServer`]), which
//! answers with the saved GitHub token or shows a dialog, and the helper prints the answer for git.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::constants::app::{ASKPASS_SOCKET_ENV, ASKPASS_TOKEN_ENV, CONFIG_DIR_NAME};
use crate::constants::github::GITHUB_HOST;
use crate::git::runner::{self, AskpassEnvironment};

const HELPER_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// The signed-in GitHub account, shared with the socket thread so it can answer git on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubAccount {
    pub login: String,
    pub token: String,
}

pub type SharedAccount = Arc<Mutex<Option<GitHubAccount>>>;

#[derive(Serialize, Deserialize)]
struct HelperQuestion {
    token: String,
    prompt: String,
}

#[derive(Serialize, Deserialize)]
struct HelperAnswer {
    answer: Option<String>,
}

/// A prompt waiting for the user. Sending `None` cancels it, and git fails with an auth error.
pub struct AskpassRequest {
    pub prompt: String,
    reply: Sender<Option<String>>,
}

impl AskpassRequest {
    pub fn answer(self, answer: Option<String>) {
        let _ = self.reply.send(answer);
    }
}

/// True for prompts whose answer must be hidden while typing.
pub fn is_secret_prompt(prompt: &str) -> bool {
    let prompt = prompt.to_lowercase();
    ["password", "passphrase", "token", "pin"].iter().any(|word| prompt.contains(word))
}

/// Answers git's own GitHub prompts with the saved account, so a signed-in user is never asked.
pub fn automatic_answer(prompt: &str, account: Option<&GitHubAccount>) -> Option<String> {
    let account = account?;
    let lower = prompt.to_lowercase();
    if !lower.contains(&format!("//{GITHUB_HOST}")) && !lower.contains(&format!("@{GITHUB_HOST}")) {
        return None;
    }
    if lower.starts_with("username") {
        Some(account.login.clone())
    } else if lower.starts_with("password") {
        Some(account.token.clone())
    } else {
        None
    }
}

pub struct AskpassServer {
    requests: Receiver<AskpassRequest>,
    socket_path: PathBuf,
}

impl AskpassServer {
    /// Listens on a socket in the user's private runtime folder and tells the git runner to use it.
    pub fn start(repaint: egui::Context, account: SharedAccount) -> std::io::Result<Self> {
        Self::start_with_helper(repaint, account, std::env::current_exe()?)
    }

    /// Like [`Self::start`], with an explicit helper executable (tests use the built app binary).
    pub fn start_with_helper(
        repaint: egui::Context,
        account: SharedAccount,
        helper: PathBuf,
    ) -> std::io::Result<Self> {
        let folder = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
        remove_stale_sockets(&folder);
        let socket_path = folder.join(format!("{CONFIG_DIR_NAME}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)?;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;

        let secret = random_token();
        runner::configure_askpass(AskpassEnvironment {
            executable: helper,
            socket: socket_path.clone(),
            token: secret.clone(),
        });

        let (sender, requests) = channel();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let sender = sender.clone();
                let repaint = repaint.clone();
                let account = Arc::clone(&account);
                let secret = secret.clone();
                std::thread::spawn(move || serve(stream, &secret, &account, &sender, &repaint));
            }
        });
        Ok(Self { requests, socket_path })
    }

    pub fn next_request(&self) -> Option<AskpassRequest> {
        self.requests.try_recv().ok()
    }
}

impl Drop for AskpassServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

fn serve(
    stream: UnixStream,
    secret: &str,
    account: &SharedAccount,
    requests: &Sender<AskpassRequest>,
    repaint: &egui::Context,
) {
    let _ = stream.set_read_timeout(Some(HELPER_READ_TIMEOUT));
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let Ok(question) = serde_json::from_str::<HelperQuestion>(&line) else { return };
    if question.token != secret {
        return;
    }
    let saved = account.lock().ok().and_then(|guard| guard.clone());
    let answer = match automatic_answer(&question.prompt, saved.as_ref()) {
        Some(answer) => Some(answer),
        None => {
            let (reply, response) = channel();
            if requests.send(AskpassRequest { prompt: question.prompt, reply }).is_err() {
                return;
            }
            repaint.request_repaint();
            response.recv().ok().flatten()
        }
    };
    let mut stream = stream;
    if let Ok(text) = serde_json::to_string(&HelperAnswer { answer }) {
        let _ = writeln!(stream, "{text}");
    }
}

/// When git started this executable as its askpass helper, answers and returns the exit code.
/// Returns `None` for a normal launch.
pub fn run_helper() -> Option<i32> {
    let socket = std::env::var_os(ASKPASS_SOCKET_ENV)?;
    let token = std::env::var(ASKPASS_TOKEN_ENV).ok()?;
    let prompt = std::env::args().nth(1).unwrap_or_default();
    let ask = || -> std::io::Result<Option<String>> {
        let mut stream = UnixStream::connect(&socket)?;
        let question = serde_json::to_string(&HelperQuestion { token, prompt })?;
        writeln!(stream, "{question}")?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line)?;
        Ok(serde_json::from_str::<HelperAnswer>(&line)?.answer)
    };
    Some(match ask() {
        Ok(Some(answer)) => {
            println!("{answer}");
            0
        }
        _ => 1,
    })
}

/// Removes sockets left by copies of the app that were killed before they could clean up.
fn remove_stale_sockets(folder: &std::path::Path) {
    // Liveness is checked through /proc, which only Linux has.
    if !cfg!(target_os = "linux") {
        return;
    }
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = name
            .to_str()
            .and_then(|name| name.strip_prefix(&format!("{CONFIG_DIR_NAME}-")))
            .and_then(|rest| rest.strip_suffix(".sock"))
            .and_then(|pid| pid.parse::<u32>().ok());
        if let Some(pid) = pid
            && !std::path::Path::new(&format!("/proc/{pid}")).exists()
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// A random value that proves a connection comes from a helper this app started.
fn random_token() -> String {
    let mut bytes = [0u8; 16];
    let read = std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes));
    if read.is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_nanos())
            .unwrap_or_default();
        bytes = (nanos ^ u128::from(std::process::id())).to_le_bytes();
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> GitHubAccount {
        GitHubAccount { login: "octo".into(), token: "secret".into() }
    }

    #[test]
    fn answers_github_prompts_only() {
        let account = account();
        let user = "Username for 'https://github.com': ";
        let pass = "Password for 'https://octo@github.com': ";
        assert_eq!(automatic_answer(user, Some(&account)).as_deref(), Some("octo"));
        assert_eq!(automatic_answer(pass, Some(&account)).as_deref(), Some("secret"));
        assert_eq!(automatic_answer("Username for 'https://gitlab.com': ", Some(&account)), None);
        assert_eq!(automatic_answer(user, None), None);
        assert_eq!(
            automatic_answer("Enter passphrase for key '/home/x/.ssh/id_ed25519': ", Some(&account)),
            None
        );
    }

    #[test]
    fn hides_secret_prompts() {
        assert!(is_secret_prompt("Password for 'https://github.com': "));
        assert!(is_secret_prompt("Enter passphrase for key 'id_ed25519': "));
        assert!(!is_secret_prompt("Username for 'https://github.com': "));
    }
}
