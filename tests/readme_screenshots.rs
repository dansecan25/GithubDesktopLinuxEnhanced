//! Renders the README screenshots from a mock project, so they never show real data.
//!
//! `GDE_README_SCREENSHOTS=docs/screenshots cargo test --test readme_screenshots -- --ignored`
//!
//! Needs a GPU or a software Vulkan driver (lavapipe).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use github_desktop_enhanced::app::DesktopApp;
use github_desktop_enhanced::services::settings::{Settings, ThemeChoice};

const SIZE: [f32; 2] = [1280.0, 800.0];
const NARROW: [f32; 2] = [400.0, 800.0];
const DAY: u64 = 24 * 60 * 60;

/// (path, content)
type MockFile = (&'static str, &'static str);
/// (author, email, days ago, summary, body, files)
type MockCommit = (&'static str, &'static str, u64, &'static str, &'static str, &'static [MockFile]);

/// Fictional people and project.
const COMMITS: &[MockCommit] = &[
    (
        "Alex Rivera",
        "alex@example.com",
        40,
        "Initial project structure",
        "",
        &[
            ("README.md", "# Weather Station\n\nReads sensors and serves a small dashboard.\n"),
            (
                "src/main.rs",
                "mod sensors;\n\nfn main() {\n    let reading = sensors::read();\n    println!(\"{reading:?}\");\n}\n",
            ),
            (
                "src/sensors.rs",
                "#[derive(Debug)]\npub struct Reading {\n    pub celsius: f32,\n}\n\npub fn read() -> Reading {\n    Reading { celsius: 21.5 }\n}\n",
            ),
        ],
    ),
    (
        "Sam Chen",
        "sam@example.com",
        31,
        "Add humidity to sensor readings",
        "The DHT22 reports both values in one read.",
        &[(
            "src/sensors.rs",
            "#[derive(Debug)]\npub struct Reading {\n    pub celsius: f32,\n    pub humidity: f32,\n}\n\npub fn read() -> Reading {\n    Reading { celsius: 21.5, humidity: 48.0 }\n}\n",
        )],
    ),
    (
        "Alex Rivera",
        "alex@example.com",
        24,
        "Serve readings as JSON",
        "",
        &[(
            "src/server.rs",
            "pub fn route(path: &str) -> Option<String> {\n    match path {\n        \"/readings\" => Some(\"[]\".to_owned()),\n        _ => None,\n    }\n}\n",
        )],
    ),
    (
        "Priya Natarajan",
        "priya@example.com",
        17,
        "Document the wiring for the sensors",
        "",
        &[(
            "docs/wiring.md",
            "# Wiring\n\n| Sensor pin | Board pin |\n|---|---|\n| VCC | 3.3V |\n| DATA | GPIO4 |\n| GND | GND |\n",
        )],
    ),
    (
        "Sam Chen",
        "sam@example.com",
        9,
        "Average the last five readings",
        "Smooths out spikes from a noisy sensor.",
        &[(
            "src/average.rs",
            "pub fn average(values: &[f32]) -> f32 {\n    values.iter().sum::<f32>() / values.len() as f32\n}\n",
        )],
    ),
    (
        "Alex Rivera",
        "alex@example.com",
        3,
        "Show the temperature trend on the dashboard",
        "",
        &[(
            "web/index.html",
            "<!doctype html>\n<title>Weather Station</title>\n<canvas id=\"trend\"></canvas>\n",
        )],
    ),
];

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

fn git_as(dir: &Path, name: &str, email: &str, days_ago: u64, args: &[&str]) {
    let date = format!("@{} +0000", now() - days_ago * DAY - 3600);
    let status = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            &format!("user.name={name}"),
            "-c",
            &format!("user.email={email}"),
            "-c",
            "commit.gpgsign=false",
        ])
        .args(["-c", "tag.gpgsign=false"])
        .args(args)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn git(dir: &Path, args: &[&str]) {
    git_as(dir, "Alex Rivera", "alex@example.com", 0, args);
}

fn write(repo: &Path, file: &str, content: &str) {
    let path = repo.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn mock_project(root: &Path) -> PathBuf {
    let repo = root.join("weather-station");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for (index, (name, email, days, summary, body, files)) in COMMITS.iter().enumerate() {
        for (file, content) in *files {
            write(&repo, file, content);
        }
        git_as(&repo, name, email, *days, &["add", "--all"]);
        let mut args = vec!["commit", "-q", "-m", summary];
        if !body.is_empty() {
            args.extend(["-m", body]);
        }
        git_as(&repo, name, email, *days, &args);
        let tag = match index {
            2 => Some(("v0.1.0", "First release: temperature and humidity over JSON")),
            4 => Some(("v0.2.0", "Smoothed readings and wiring docs")),
            _ => None,
        };
        if let Some((tag, message)) = tag {
            git_as(&repo, name, email, *days, &["tag", "-a", tag, "-m", message]);
        }
    }

    // A GitHub remote without network access: the remote-tracking refs are written directly.
    git(&repo, &["remote", "add", "origin", "https://github.com/example/weather-station.git"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD~1"]);
    git(&repo, &["branch", "--set-upstream-to", "origin/main"]);
    git(&repo, &["branch", "feature/alerts"]);
    git(&repo, &["update-ref", "refs/remotes/origin/feature/alerts", "HEAD~2"]);
    git(&repo, &["branch", "fix/sensor-timeout", "HEAD~1"]);
    git(&repo, &["update-ref", "refs/remotes/origin/dashboard-dark-mode", "HEAD~3"]);

    // Work in progress.
    write(
        &repo,
        "src/sensors.rs",
        "#[derive(Debug)]\npub struct Reading {\n    pub celsius: f32,\n    pub humidity: f32,\n    pub pressure_hpa: f32,\n}\n\n/// Reads every sensor once. Retries a timed-out read before giving up.\npub fn read() -> Reading {\n    Reading { celsius: 21.5, humidity: 48.0, pressure_hpa: 1013.2 }\n}\n",
    );
    write(
        &repo,
        "src/alerts.rs",
        "/// Raises an alert when the temperature leaves the safe range.\npub fn check(celsius: f32) -> Option<&'static str> {\n    if celsius > 35.0 {\n        Some(\"Too hot\")\n    } else if celsius < 0.0 {\n        Some(\"Freezing\")\n    } else {\n        None\n    }\n}\n",
    );
    write(
        &repo,
        "docs/wiring.md",
        "# Wiring\n\n| Sensor pin | Board pin |\n|---|---|\n| VCC | 3.3V |\n| DATA | GPIO4 |\n| GND | GND |\n| SDA (BMP280) | GPIO2 |\n| SCL (BMP280) | GPIO3 |\n",
    );
    std::fs::remove_file(repo.join("web/index.html")).unwrap();
    repo
}

fn settle(harness: &mut Harness<'_, DesktopApp>) {
    for _ in 0..30 {
        harness.step();
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn button<'a>(harness: &'a Harness<'_, DesktopApp>, starts_with: &str) -> egui_kittest::Node<'a> {
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().is_some_and(|text| text.starts_with(starts_with)))
        .unwrap_or_else(|| panic!("no button starting with {starts_with}"))
}

fn save(harness: &mut Harness<'_, DesktopApp>, dir: &Path, name: &str) {
    let image = harness.render().expect("render");
    image.save(dir.join(format!("{name}.png"))).unwrap();
}

fn open_app(repo: &Path, theme: ThemeChoice) -> Harness<'static, DesktopApp> {
    let settings = Settings {
        repositories: vec![repo.to_path_buf(), repo.with_file_name("home-automation")],
        selected_repository: Some(repo.to_path_buf()),
        theme,
        github_login: None,
        ..Settings::default()
    };
    let egui_theme = if theme == ThemeChoice::Light { egui::Theme::Light } else { egui::Theme::Dark };
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_theme(egui_theme)
        .build_eframe(move |cc| DesktopApp::new(cc, settings));
    settle(&mut harness);
    harness
}

#[test]
#[ignore = "renders README screenshots; needs a GPU or software Vulkan"]
fn readme_screenshots() {
    let Ok(out) = std::env::var("GDE_README_SCREENSHOTS") else { return };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let root = std::env::temp_dir().join(format!("gde-readme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("home-automation")).unwrap();
    // SAFETY: this test binary runs a single test, before any other thread reads the environment.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", root.join("config")) };
    let repo = mock_project(&root);
    git(&root.join("home-automation"), &["init", "-q", "-b", "main"]);

    let mut dark = open_app(&repo, ThemeChoice::Dark);
    button(&dark, "sensors.rs").click();
    settle(&mut dark);
    save(&mut dark, &out, "changes");

    button(&dark, "alerts.rs").click_secondary();
    settle(&mut dark);
    save(&mut dark, &out, "file-menu");
    dark.key_press(egui::Key::Escape);
    settle(&mut dark);

    button(&dark, "Current branch").click();
    settle(&mut dark);
    dark.get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().as_deref() == Some("feature/alerts"))
        .unwrap()
        .click_secondary();
    settle(&mut dark);
    save(&mut dark, &out, "branch-menu");
    dark.key_press(egui::Key::Escape);
    dark.key_press(egui::Key::Escape);
    settle(&mut dark);

    dark.get_by_label("History").click();
    settle(&mut dark);
    button(&dark, "Show the temperature trend").click();
    settle(&mut dark);
    button(&dark, "Show the temperature trend").click_secondary();
    settle(&mut dark);
    dark.get_by_label("Create tag…").click();
    settle(&mut dark);
    save(&mut dark, &out, "create-tag");
    dark.get_by_label("Cancel").click();
    settle(&mut dark);

    dark.get_by_label("Menu").click();
    settle(&mut dark);
    dark.get_by_label("Preferences…").click();
    settle(&mut dark);
    save(&mut dark, &out, "preferences");

    let mut light = open_app(&repo, ThemeChoice::Light);
    light.get_by_label("History").click();
    settle(&mut light);
    button(&light, "Average the last five readings").click();
    settle(&mut light);
    button(&light, "average.rs").click();
    settle(&mut light);
    save(&mut light, &out, "history");

    light.get_by_label("Changes (4)").click();
    light.set_size(NARROW.into());
    settle(&mut light);
    save(&mut light, &out, "narrow");

    let _ = std::fs::remove_dir_all(&root);
}
