//! The credential bridge end to end: the real binary, started the way git starts its askpass
//! helper, talking to an in-process server.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use github_desktop_enhanced::constants::app::{ASKPASS_SOCKET_ENV, ASKPASS_TOKEN_ENV};
use github_desktop_enhanced::services::askpass::{AskpassServer, GitHubAccount};

/// Finds the socket and token the server handed to the git runner, through a real git call.
fn helper_environment() -> (String, String) {
    // `git var GIT_ASKPASS` would only show the path; the runner's environment is what matters,
    // so run a tiny git alias that prints the variables the runner sets.
    let dir = std::env::temp_dir();
    let output = github_desktop_enhanced::git::runner::run(
        &dir,
        &["-c", "alias.env=!printenv", "env", ASKPASS_SOCKET_ENV, ASKPASS_TOKEN_ENV],
    )
    .expect("git env");
    let mut lines = output.lines();
    (lines.next().unwrap().to_owned(), lines.next().unwrap().to_owned())
}

fn ask(socket: &str, token: &str, prompt: &str) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_github-desktop-enhanced"))
        .arg(prompt)
        .env(ASKPASS_SOCKET_ENV, socket)
        .env(ASKPASS_TOKEN_ENV, token)
        .output()
        .unwrap();
    (output.status.success(), String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
fn helper_answers_from_account_or_asks_the_app() {
    let account = Arc::new(Mutex::new(Some(GitHubAccount { login: "octo".into(), token: "tok".into() })));
    let helper = env!("CARGO_BIN_EXE_github-desktop-enhanced").into();
    let server =
        AskpassServer::start_with_helper(egui::Context::default(), Arc::clone(&account), helper).unwrap();
    let (socket, token) = helper_environment();

    // Answered automatically from the signed-in account.
    assert_eq!(ask(&socket, &token, "Username for 'https://github.com': "), (true, "octo".into()));
    assert_eq!(ask(&socket, &token, "Password for 'https://octo@github.com': "), (true, "tok".into()));

    // Real git uses the bridge for HTTPS credentials even with terminal prompts turned off.
    let filled = github_desktop_enhanced::git::runner::run_with_input(
        &std::env::temp_dir(),
        &["-c", "credential.helper=", "credential", "fill"],
        Some("protocol=https\nhost=github.com\n\n"),
    )
    .unwrap();
    assert!(filled.contains("username=octo") && filled.contains("password=tok"), "{filled}");

    // Anything else goes to the app, which answers like the dialog would. The thread keeps the
    // server alive until it is joined.
    let answering = std::thread::spawn(move || {
        let started = Instant::now();
        let mut answered = 0;
        while answered < 2 && started.elapsed() < Duration::from_secs(20) {
            if let Some(request) = server.next_request() {
                let answer = if request.prompt.contains("cancel") { None } else { Some("typed".to_owned()) };
                request.answer(answer);
                answered += 1;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        (server, answered)
    });
    assert_eq!(ask(&socket, &token, "Enter passphrase for key 'id_ed25519': "), (true, "typed".into()));
    let (ok, _) = ask(&socket, &token, "Password, cancel this one: ");
    assert!(!ok, "a cancelled prompt must make the helper fail");

    // A wrong token is refused without asking anyone.
    let (ok, _) = ask(&socket, "wrong", "Username for 'https://github.com': ");
    assert!(!ok);
    let (_server, answered) = answering.join().unwrap();
    assert_eq!(answered, 2);
}
