//! Renders the pull request pages with made-up GitHub data, so they can be checked without an
//! account. Needs a GPU or lavapipe, like `ui_screens`:
//! `GDE_SCREENSHOT_DIR=/some/folder cargo test --test pull_screens -- --ignored`

use std::path::Path;

use eframe::egui;
use egui_kittest::Harness;
use github_desktop_enhanced::git::parse::parse_diff;
use github_desktop_enhanced::models::{
    ChangeKind, Checks, DiffSide, FileChange, LineTarget, MergeMethod, Mergeability, PullComment, PullDetail,
    PullFile, PullList, PullRequest, RepoSnapshot, ReviewComment,
};
use github_desktop_enhanced::services::settings::{Settings, ThemeChoice};
use github_desktop_enhanced::stores::{Loadable, RepoStore};
use github_desktop_enhanced::ui::actions::{PullView, Tab};
use github_desktop_enhanced::ui::theme;
use github_desktop_enhanced::ui::views::{ViewContext, repository_screen};

const WIDE: [f32; 2] = [1200.0, 760.0];
const TALL: [f32; 2] = [1200.0, 1200.0];

fn pull() -> PullRequest {
    PullRequest {
        number: 42,
        title: "Read the port from the environment".into(),
        body: "The server used a fixed port. Now `PORT` overrides it, and the default stays 8080.".into(),
        author: "sam".into(),
        head_branch: "env-port".into(),
        base_branch: "main".into(),
        head_sha: "abc123".into(),
        same_repository: true,
        draft: false,
        url: "https://github.com/owner/demo/pull/42".into(),
        created_at: "2026-11-20T09:00:00Z".into(),
    }
}

fn detail() -> PullDetail {
    let patch = "@@ -1,6 +1,8 @@\n use std::env;\n \n fn port() -> u16 {\n-    8080\n+    env::var(\"PORT\")\n\
                 +        .ok()\n+        .and_then(|value| value.parse().ok())\n+        .unwrap_or(8080)\n }";
    let file = |path: &str, kind, patch: Option<&str>| PullFile {
        change: FileChange { path: path.into(), original_path: None, kind },
        additions: 4,
        deletions: 1,
        diff: patch.map(parse_diff).unwrap_or_default(),
        diff_missing: patch.is_none(),
    };
    let comment = |id, reply_to, line, side, author: &str, body: &str| ReviewComment {
        id,
        reply_to,
        path: "src/server.rs".into(),
        line,
        side,
        author: author.into(),
        body: body.into(),
        created_at: "2026-11-21T10:00:00Z".into(),
    };
    let entry = |author: &str, body: &str, verdict: Option<&str>| PullComment {
        author: author.into(),
        body: body.into(),
        created_at: "2026-11-21T11:00:00Z".into(),
        verdict: verdict.map(str::to_owned),
    };
    PullDetail {
        comments: vec![
            entry("ana", "Thanks! Could the default live in a constant?", None),
            entry("lee", "Looks good to me.", Some("approved")),
        ],
        checks: Checks { passed: 3, failed: 0, pending: 1 },
        files: vec![
            file("src/server.rs", ChangeKind::Modified, Some(patch)),
            file("assets/logo.png", ChangeKind::Added, None),
        ],
        review_comments: vec![
            comment(
                1,
                None,
                Some(5),
                DiffSide::Right,
                "ana",
                "What happens when PORT isn't a number? A warning in \
                the log would help whoever set it wrong, instead of silently using 8080.",
            ),
            comment(2, Some(1), Some(5), DiffSide::Right, "sam", "Good point, I'll log it."),
            comment(3, None, Some(4), DiffSide::Left, "lee", "Nice to see the magic number go."),
            comment(4, None, None, DiffSide::Right, "ana", "This was about an older version."),
        ],
        mergeability: Mergeability { mergeable: Some(true), state: "unstable".into(), merged: false },
        node_id: "PR_node".into(),
    }
}

fn store(root: &Path) -> RepoStore {
    let mut store = RepoStore::new(root.to_path_buf());
    let snapshot = RepoSnapshot {
        remote: Some("origin".into()),
        remote_url: Some("https://github.com/owner/demo.git".into()),
        ..RepoSnapshot::default()
    };
    store.snapshot = Loadable::Ready(snapshot);
    let other = PullRequest { number: 41, title: "Draft: dark mode".into(), draft: true, ..pull() };
    store.pulls = Loadable::Ready(PullList {
        pulls: vec![pull(), other],
        default_branch: "main".into(),
        merge_methods: vec![MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase],
    });
    store.selected_pull = Some(42);
    store.pull_detail = Loadable::Ready(detail());
    store
}

fn save(harness: &mut Harness<'_>, name: &str) {
    let image = harness.render().expect("render");
    if let Ok(dir) = std::env::var("GDE_SCREENSHOT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        image.save(Path::new(&dir).join(format!("{name}.png"))).unwrap();
    }
}

fn render(name: &str, size: [f32; 2], store: RepoStore) {
    let settings = Settings { github_login: Some("sam".into()), ..Settings::default() };
    let mut store = store;
    let mut fonts_installed = false;
    let mut harness = Harness::builder().with_size(size).with_theme(egui::Theme::Dark).build_ui(move |ui| {
        if !fonts_installed {
            theme::install_fonts(ui.ctx());
            fonts_installed = true;
        }
        theme::apply(ui.ctx(), ThemeChoice::Dark);
        let view = ViewContext {
            settings: &settings,
            editors: &[],
            busy: None,
            progress: None,
            can_cancel: false,
            open_in_label: "Open in editor",
        };
        let mut actions = Vec::new();
        repository_screen(ui, &mut store, Tab::PullRequests, true, &view, &mut actions);
    });
    for _ in 0..5 {
        harness.step();
    }
    save(&mut harness, name);
}

#[test]
#[ignore = "needs a GPU or software Vulkan; run with --ignored"]
fn renders_pull_request_pages() {
    let root = std::env::temp_dir();
    render("pull-conversation", WIDE, store(&root));
    // Tall enough to show the review form under the conversation.
    render("pull-review", TALL, store(&root));

    let mut files = store(&root);
    files.pull_view = PullView::Files;
    files.selected_pull_file = Some("src/server.rs".into());
    render("pull-files", WIDE, files);

    let mut commenting = store(&root);
    commenting.pull_view = PullView::Files;
    commenting.selected_pull_file = Some("src/server.rs".into());
    let target = LineTarget { path: "src/server.rs".into(), line: 8, side: DiffSide::Right, reply_to: None };
    commenting.line_comment = Some((target, "Should this be documented in the README?".into()));
    render("pull-line-comment", WIDE, commenting);
}
