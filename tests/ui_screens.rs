//! Drives the real app headlessly and renders its main screens.
//!
//! Needs a GPU or a software Vulkan driver (lavapipe), so it is ignored by default:
//! `GDE_SCREENSHOT_DIR=/some/folder cargo test --test ui_screens -- --ignored`
//! writes one PNG per screen into that folder.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use github_desktop_enhanced::app::DesktopApp;
use github_desktop_enhanced::services::settings::{Settings, ThemeChoice};

const WIDE: [f32; 2] = [1200.0, 760.0];
const NARROW: [f32; 2] = [380.0, 760.0];

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn demo_repository(root: &Path) -> PathBuf {
    let repo = root.join("demo-project");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("main.rs"), "fn main() {\n    println!(\"hello\");\n}\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "First commit", "-m", "Sets up the project."]);
    git(&repo, &["tag", "-a", "v0.1.0", "-m", "First release"]);
    std::fs::write(repo.join("main.rs"), "fn main() {\n    println!(\"hello, world\");\n}\n").unwrap();
    std::fs::write(repo.join("notes.txt"), "remember to write docs\n").unwrap();
    // A long path must be cut with "…", not widen the sidebar.
    let deep = repo.join("portfolio/public/images/projects/github-desktop-enhanced");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("branch-menu.png"), "not really an image\n").unwrap();
    repo
}

/// Clicks the list row (a button) whose label starts with `label`.
fn click_row(harness: &Harness<'_, DesktopApp>, label: &str) {
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().is_some_and(|text| text.starts_with(label)))
        .unwrap_or_else(|| panic!("no row starting with {label}"))
        .click();
}

fn right_click_row(harness: &Harness<'_, DesktopApp>, label: &str) {
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().is_some_and(|text| text.starts_with(label)))
        .unwrap_or_else(|| panic!("no row starting with {label}"))
        .click_secondary();
}

/// Steps until background git tasks have had time to finish.
fn settle(harness: &mut Harness<'_, DesktopApp>) {
    for _ in 0..30 {
        harness.step();
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn save(harness: &mut Harness<'_, DesktopApp>, name: &str) {
    let image = harness.render().expect("render");
    if let Ok(dir) = std::env::var("GDE_SCREENSHOT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        image.save(Path::new(&dir).join(format!("{name}.png"))).unwrap();
    }
}

/// Right-click menus and the new dialogs, in the wide layout.
fn dialogs_and_menus(harness: &mut Harness<'_, DesktopApp>, repo: &Path) {
    // Back first: clicks land on the layout from the last frame, before any resize.
    harness.get_by_label("← Back").click();
    settle(harness);
    harness.set_size(WIDE.into());
    settle(harness);
    right_click_row(harness, "main.rs");
    settle(harness);
    save(harness, "menu-file");
    harness.key_press(egui::Key::Escape);
    settle(harness);

    harness.get_by_label("Menu").click();
    settle(harness);
    harness.get_by_label("Preferences…").click();
    settle(harness);
    save(harness, "dialog-preferences");
    harness.get_by_label("Close").click();
    settle(harness);

    click_row(harness, "Publish repository");
    settle(harness);
    save(harness, "dialog-publish");
    harness.get_by_label("Cancel").click();
    settle(harness);

    // A touchpad that sends a left click with the right click must only open the right-click menu.
    let repository_button = |harness: &Harness<'_, DesktopApp>| {
        harness
            .get_all_by_role(egui::accesskit::Role::Button)
            .find(|node| {
                node.accesskit_node().label().is_some_and(|text| text.starts_with("Current repository"))
            })
            .unwrap()
            .rect()
    };
    let center = repository_button(harness).center();
    harness.event(egui::Event::PointerMoved(center));
    for button in [egui::PointerButton::Primary, egui::PointerButton::Secondary] {
        let press = |pressed| egui::Event::PointerButton {
            pos: center,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        harness.event(press(true));
        harness.event(press(false));
    }
    settle(harness);
    assert!(harness.query_by_label("Repositories").is_none(), "the left-click dropdown opened too");
    assert!(harness.query_by_label("Copy full path").is_some(), "the right-click menu didn't open");
    save(harness, "menu-repository");
    harness.key_press(egui::Key::Escape);
    settle(harness);

    git(repo, &["remote", "add", "origin", "https://github.com/example/demo-project.git"]);
    harness.key_press(egui::Key::F5);
    settle(harness);
    click_row(harness, "Current branch");
    settle(harness);
    // The branch row is labelled exactly "main" (the file row "main.rs" starts the same way).
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().as_deref() == Some("main"))
        .expect("branch row")
        .click_secondary();
    settle(harness);
    save(harness, "menu-branch");
    harness.key_press(egui::Key::Escape);
    settle(harness);

    // A branch one commit ahead, made without touching the working tree.
    let ahead = Command::new("git")
        .current_dir(repo)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.invalid"])
        .args(["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "Feature work"])
        .output()
        .unwrap();
    git(repo, &["branch", "feature", String::from_utf8_lossy(&ahead.stdout).trim()]);
    harness.key_press(egui::Key::Escape);
    harness.key_press(egui::Key::F5);
    settle(harness);
    if harness.query_by_label("Choose a branch to merge into main").is_none() {
        click_row(harness, "Current branch");
        settle(harness);
    }
    harness.get_by_label("Choose a branch to merge into main").click();
    settle(harness);
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().as_deref() == Some("feature"))
        .expect("feature row in the merge dialog")
        .click();
    settle(harness);
    assert!(
        harness.query_by_label("This will merge 1 commit from feature into main.").is_some(),
        "merge preview text"
    );
    save(harness, "dialog-merge");
    harness.key_press(egui::Key::Escape);
    settle(harness);

    // Switching with changes asks what to do with them, as GitHub Desktop does.
    click_row(harness, "Current branch");
    settle(harness);
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| node.accesskit_node().label().as_deref() == Some("feature"))
        .expect("feature row in the branch menu")
        .click();
    settle(harness);
    assert!(harness.query_by_label("Leave my changes on main").is_some(), "switch dialog");
    save(harness, "dialog-switch-branch");
    harness.key_press(egui::Key::Escape);
    settle(harness);

    harness.get_by_label("History").click();
    settle(harness);
    right_click_row(harness, "First commit");
    settle(harness);
    save(harness, "menu-commit");
    harness.get_by_label("Create tag…").click();
    settle(harness);
    save(harness, "dialog-tag");
}

#[test]
#[ignore = "needs a GPU or software Vulkan; run with --ignored"]
fn renders_main_screens() {
    let root = std::env::temp_dir().join(format!("gde-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // SAFETY: this test binary runs a single test, before any other thread reads the environment.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", root.join("config")) };

    let welcome_settings = Settings::default();
    let mut welcome =
        Harness::builder().with_size(WIDE).build_eframe(|cc| DesktopApp::new(cc, welcome_settings));
    settle(&mut welcome);
    save(&mut welcome, "welcome");

    let repo = demo_repository(&root);
    for theme in [ThemeChoice::Dark, ThemeChoice::Light] {
        let settings = Settings {
            repositories: vec![repo.clone()],
            selected_repository: Some(repo.clone()),
            theme,
            ..Settings::default()
        };
        let suffix = format!("{theme:?}").to_lowercase();
        // The harness applies its own theme after the app starts, so it is set here as well.
        let egui_theme = if theme == ThemeChoice::Light { egui::Theme::Light } else { egui::Theme::Dark };
        let mut harness = Harness::builder()
            .with_size(WIDE)
            .with_theme(egui_theme)
            .build_eframe(|cc| DesktopApp::new(cc, settings));
        settle(&mut harness);
        click_row(&harness, "main.rs");
        settle(&mut harness);
        save(&mut harness, &format!("changes-{suffix}"));

        harness.get_by_label("History").click();
        settle(&mut harness);
        click_row(&harness, "First commit");
        settle(&mut harness);
        click_row(&harness, "main.rs");
        settle(&mut harness);
        save(&mut harness, &format!("history-{suffix}"));

        harness.get_by_label_contains("Changes").click();
        settle(&mut harness);
        harness.set_size(NARROW.into());
        settle(&mut harness);
        save(&mut harness, &format!("narrow-{suffix}"));

        click_row(&harness, "main.rs");
        settle(&mut harness);
        save(&mut harness, &format!("narrow-diff-{suffix}"));

        if theme == ThemeChoice::Dark {
            dialogs_and_menus(&mut harness, &repo);
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
