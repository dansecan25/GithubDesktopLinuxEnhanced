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

/// A 96×64 picture: a coloured circle on transparency, so the checkerboard shows.
fn logo(color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_fn(96, 64, |x, y| {
        let (dx, dy) = (x as f32 - 48.0, y as f32 - 32.0);
        if dx * dx + dy * dy < 28.0 * 28.0 { image::Rgba(color) } else { image::Rgba([0, 0, 0, 0]) }
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

fn demo_repository(root: &Path) -> PathBuf {
    let repo = root.join("demo-project");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("main.rs"), "fn main() {\n    println!(\"hello\");\n}\n").unwrap();
    std::fs::write(repo.join("logo.png"), logo([0x1f, 0x6f, 0xeb, 0xff])).unwrap();
    // The tag dialog checks the tag against this version.
    std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "First commit", "-m", "Sets up the project."]);
    git(&repo, &["tag", "-a", "v0.1.0", "-m", "First release"]);
    std::fs::write(repo.join("main.rs"), "fn main() {\n    println!(\"hello, world\");\n}\n").unwrap();
    std::fs::write(repo.join("notes.txt"), "remember to write docs\n").unwrap();
    std::fs::write(repo.join("logo.png"), logo([0x2d, 0xa4, 0x4e, 0x80])).unwrap();
    // A long path must be cut with "…", not widen the sidebar.
    let deep = repo.join("portfolio/public/images/projects/github-desktop-enhanced");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("branch-menu.png"), "not really an image\n").unwrap();
    // Long enough to scroll: the whole height of the diff must stay filled.
    let long: String = (1..=300).map(|n| format!("line {n}\n")).collect();
    std::fs::write(repo.join("long.txt"), long).unwrap();
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

/// Drags over the removed and added line of main.rs's diff, then copies them with Ctrl+C.
fn select_and_copy_diff_lines(harness: &mut Harness<'_, DesktopApp>) {
    let (from, to) = (egui::pos2(600.0, 201.0), egui::pos2(600.0, 218.0));
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(from));
    harness.event(button(from, true));
    harness.step();
    harness.event(egui::Event::PointerMoved(to));
    harness.step();
    harness.event(button(to, false));
    harness.step();
    harness.event(egui::Event::Copy);
    harness.step();
    let copied: Vec<&String> = harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(copied, ["    println!(\"hello\");\n    println!(\"hello, world\");"]);
    save(harness, "diff-selection");

    // Clicking the removed line's gutter leaves it out of the commit.
    let gutter = egui::pos2(330.0, 201.0);
    harness.event(egui::Event::PointerMoved(gutter));
    harness.event(button(gutter, true));
    harness.step();
    harness.event(button(gutter, false));
    harness.step();
    harness.step();
    save(harness, "lines-picked");
}

/// Scrolls to the end of a long diff, then opens the co-authors field.
fn long_diff_and_co_authors(harness: &mut Harness<'_, DesktopApp>) {
    click_row(harness, "long.txt");
    settle(harness);
    harness.event(egui::Event::PointerMoved(egui::pos2(700.0, 400.0)));
    for _ in 0..40 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -10.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
        });
        harness.step();
    }
    settle(harness);
    save(harness, "long-diff-end");
    harness.get_by_label("Add co-authors").click();
    settle(harness);
    save(harness, "co-authors");
    harness.event(egui::Event::PointerMoved(egui::pos2(60.0, 24.0)));
    harness.step();
    harness.step();
    save(harness, "toolbar-hover");
    click_row(harness, "main.rs");
    settle(harness);
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
    // The branch row is labelled "main" plus its last commit time ("main just now"). The file row
    // "main.rs" starts the same way, so the first word must be exactly "main".
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| {
            node.accesskit_node().label().is_some_and(|label| label.split_whitespace().next() == Some("main"))
        })
        .expect("branch row")
        .click_secondary();
    // Each branch shows how long ago its last commit was, as in GitHub Desktop.
    assert!(
        harness.query_by_label_contains("ago").is_some()
            || harness.query_by_label_contains("just now").is_some()
    );
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
    harness.get_by_label("Squash and merge").click();
    settle(harness);
    assert!(harness.query_by_label("Squash and merge feature into main").is_some(), "squash button");
    save(harness, "dialog-merge-squash");
    harness.get_by_label("Rebase").click();
    settle(harness);
    // The demo repository has uncommitted changes, which a rebase refuses.
    assert!(harness.query_by_label_contains("Commit or stash your changes before rebasing").is_some());
    save(harness, "dialog-merge-rebase");
    harness.key_press(egui::Key::Escape);
    settle(harness);

    // Switching with changes asks what to do with them, as GitHub Desktop does.
    click_row(harness, "Current branch");
    settle(harness);
    harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .find(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|label| label.split_whitespace().next() == Some("feature"))
        })
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
    assert!(
        harness.query_by_label("Matches the version in Cargo.toml (0.2.0).").is_some(),
        "the dialog should offer v0.2.0 from Cargo.toml"
    );
    save(harness, "dialog-tag");
    harness.get_by_label("v0.1.1").click();
    settle(harness);
    assert!(
        harness.query_by_label_contains("Cargo.toml says 0.2.0 on this commit, not v0.1.1").is_some(),
        "mismatch warning"
    );
    save(harness, "dialog-tag-mismatch");
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
        if theme == ThemeChoice::Dark {
            click_row(&harness, "logo.png");
            settle(&mut harness);
            save(&mut harness, "image-diff");
            click_row(&harness, "branch-menu.png");
            settle(&mut harness);
            save(&mut harness, "image-unreadable");
            click_row(&harness, "main.rs");
            settle(&mut harness);
        }
        if theme == ThemeChoice::Dark {
            select_and_copy_diff_lines(&mut harness);
        }
        if theme == ThemeChoice::Dark {
            long_diff_and_co_authors(&mut harness);
        }

        harness.get_by_label("History").click();
        settle(&mut harness);
        click_row(&harness, "First commit");
        settle(&mut harness);
        click_row(&harness, "main.rs");
        settle(&mut harness);
        save(&mut harness, &format!("history-{suffix}"));

        harness.get_by_label("PRs").click();
        settle(&mut harness);
        save(&mut harness, &format!("pulls-{suffix}"));

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
    conflict_editor(&root);
    amend_and_rebase(&root);
    let _ = std::fs::remove_dir_all(&root);
}

/// A merge stopped on a conflict, shown in the conflict editor.
fn conflict_editor(root: &Path) {
    let repo = root.join("conflict-project");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("config.toml"), "[server]\nport = 8080\nhost = \"localhost\"\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "Add config"]);
    git(&repo, &["switch", "-q", "-c", "feature"]);
    std::fs::write(repo.join("config.toml"), "[server]\nport = 9090\nhost = \"localhost\"\n").unwrap();
    git(&repo, &["commit", "-q", "-am", "Use port 9090"]);
    git(&repo, &["switch", "-q", "main"]);
    std::fs::write(repo.join("config.toml"), "[server]\nport = 3000\nhost = \"localhost\"\n").unwrap();
    git(&repo, &["commit", "-q", "-am", "Use port 3000"]);
    let _ = Command::new("git").current_dir(&repo).args(["merge", "feature"]).output();

    let settings = Settings {
        repositories: vec![repo.clone()],
        selected_repository: Some(repo.clone()),
        theme: ThemeChoice::Dark,
        ..Settings::default()
    };
    let mut harness = Harness::builder().with_size(WIDE).build_eframe(|cc| DesktopApp::new(cc, settings));
    settle(&mut harness);
    click_row(&harness, "config.toml");
    settle(&mut harness);
    assert!(harness.query_by_label("Use theirs").is_some(), "conflict editor");
    save(&mut harness, "conflict-editor");
    harness.get_by_label("Use both").click();
    settle(&mut harness);
    settle(&mut harness);
    assert!(harness.query_by_label("Mark as resolved").is_some(), "resolved file");
    save(&mut harness, "conflict-resolved");
}

fn open(repo: &Path) -> Harness<'static, DesktopApp> {
    let settings = Settings {
        repositories: vec![repo.to_path_buf()],
        selected_repository: Some(repo.to_path_buf()),
        theme: ThemeChoice::Dark,
        ..Settings::default()
    };
    let mut harness = Harness::builder().with_size(WIDE).build_eframe(|cc| DesktopApp::new(cc, settings));
    settle(&mut harness);
    harness
}

/// Undoing and amending the last commit, then a rebase stopped on a conflict.
fn amend_and_rebase(root: &Path) {
    let repo = root.join("rebase-project");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("config.toml"), "[server]\nport = 8080\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "Add config"]);
    git(&repo, &["switch", "-q", "-c", "feature"]);
    std::fs::write(repo.join("config.toml"), "[server]\nport = 9090\n").unwrap();
    git(&repo, &["commit", "-q", "-am", "Use port 9090", "-m", "Co-authored-by: Ana <ana@example.com>"]);
    git(&repo, &["switch", "-q", "main"]);
    std::fs::write(repo.join("config.toml"), "[server]\nport = 3000\n").unwrap();
    git(&repo, &["commit", "-q", "-am", "Use port 3000"]);
    git(&repo, &["switch", "-q", "feature"]);

    let mut harness = open(&repo);
    assert!(harness.query_by_label_contains("Last commit: Use port 9090").is_some(), "undo row");
    save(&mut harness, "undo-commit");
    harness.get_by_label("History").click();
    settle(&mut harness);
    right_click_row(&harness, "Use port 9090");
    settle(&mut harness);
    harness.get_by_label("Amend commit…").click();
    settle(&mut harness);
    assert!(harness.query_by_label("Amend last commit").is_some(), "amend button");
    assert!(harness.query_by_label_contains("Ana ×").is_some(), "co-author kept");
    save(&mut harness, "amend-commit");
    drop(harness);

    let _ = Command::new("git").current_dir(&repo).args(["rebase", "main"]).output();
    let mut harness = open(&repo);
    click_row(&harness, "config.toml");
    settle(&mut harness);
    assert!(harness.query_by_label_contains("Rebasing feature onto main (commit 1 of 1)").is_some());
    assert!(harness.query_by_label("Continue rebase").is_some(), "continue button");
    save(&mut harness, "rebase-in-progress");
}
