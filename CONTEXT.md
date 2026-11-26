# Context

Rules and current state for anyone (person or agent) working on this repository. Features and setup
are in [README.md](README.md). Structure and internals are in [DOCUMENTATION.md](DOCUMENTATION.md).

## Rules

- The general rules in the owner's personal rules file apply: no commits, tags or pull requests by
  agents, no agent attribution, no secrets in tracked files.
- Commit messages start with a tag: `[ADD]`, `[UPDATE]`, `[FIX]`, `[CONFIG]`, `[REMOVE]`.
- All git access goes through `git/runner.rs`. Parsing goes in `git/parse.rs` with a unit test.
- Views only push `Action`s. Git work only runs in background tasks started by `app/`.
- Every user-facing string lives in `constants/text.rs`, every size or limit in `constants/`, every
  colour in `ui/theme.rs`. Reuse the shared widgets in `ui/components/` instead of styling raw egui
  widgets.
- Only use glyphs that the bundled fonts (Ubuntu and Hack) contain. The app draws a box for a
  missing glyph, but the UI test harness panics on one, which catches mistakes.
- Secrets (the GitHub token) live only in the system keychain through `services/secrets.rs`. Never
  write them to settings, logs or error messages.
- Before saying a change is done: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, and for UI changes the screenshot test in `tests/ui_screens.rs`.
- Run builds and tests through `./limited.sh`. The owner's laptop froze (2026-11-24) when the
  screenshot tests ran while swap was already full: software rendering plus a parallel build used
  up the memory, and nothing stopped it. Never run the screenshot tests right after a full build.
- On 2026-11-24 git was stopped twice by the app's 2 GB memory limit: on `main`, which had no
  `.gitignore` yet, switching branches stashed the 14 GB `target/` folder as untracked files.
  `changes::check_untracked_size` now refuses that for stashes and commits.
- Programs the app starts for the user (editors, file manager) go through
  `memory_cap::outside_cap`, so they don't count against the app's memory limit.
- Check new dependencies with `cargo audit` and keep the count low.

## Production setup

There is no server. Releases are built by `.github/workflows/release.yml` when a `vX.Y.Z` tag is
pushed, which creates a draft GitHub release for the owner to publish. The tag must match the version
in `Cargo.toml`. Nothing machine-specific belongs in this repository. Use a gitignored
`CONTEXT.local.md` for local notes.

## Known issues

- `cargo audit` reports four "unmaintained" warnings, all from eframe's dependency tree with no
  upgrade available yet: `ttf-parser` (fonts) and `zvariant`, `zvariant_derive`, `zbus_names` (the
  D-Bus stack used by AccessKit, the XDG folder picker and the keychain). No vulnerabilities are
  reported.
- A freeze after a right-click was reported once (2026-11-24), with a file just downloaded into the
  repository. It could not be reproduced headlessly. Likely causes were fixed: the folder picker no
  longer runs on the UI thread, non-UTF-8 files show as binary, and very long diff lines are cut. If
  it happens again, note the exact steps. Since then the watchdog (`services/watchdog.rs`) writes
  a report to `~/.local/state/github-desktop-enhanced/reports/` when a frame takes over 5 s: ask
  for it.
- A freeze "after some time open" was reported (2026-11-26): clicks did nothing and the app had to
  be closed. No freeze report was written and the app was idle (low CPU, under 210 MB), so the main
  thread wasn't stuck in a frame: it was stuck outside one, in presenting. The laptop has only the
  NVIDIA GPU (driver 595, Wayland), where a vsync (FIFO) present can block for good once the window
  is hidden or the screen goes off. Fixed by presenting without vsync (`services/graphics.rs`,
  `AutoNoVsync`) plus a 60 fps cap (`FrameLimiter`). Not reproduced, so not proven. The watchdog
  now also writes a `stall` report when a focused window isn't drawn for 15 s, naming the GPU; if
  the freeze comes back, ask for it. (Changing the GPU preference was tried first and dropped:
  there's no second GPU on that laptop.)
- Right-click menus are drawn by `ui/components/clicks.rs`, not egui's own context menu. egui keeps
  one popup open at a time, so its menus closed the dropdown they were opened from, and some
  touchpads send a left click with the right click. Use `right_click_menu`, `left_clicked` and
  `dropdown_menu` for any new menu or clickable row.
- Conflicts are resolved block by block in `ui/views/conflict.rs` (`git/conflicts.rs` parses the
  markers, keeping line endings). Committing is still refused while a conflicted file has conflict
  markers (`git/merge.rs`).
- Squash merges (`merge::squash`) run `git merge --squash` and commit with a message listing the
  branch's commit summaries, written to `SQUASH_MSG` so a squash stopped on conflicts shows it in
  the form. `RepoSnapshot::in_progress` tells a merge, a squash and a rebase apart. Rebases
  (`git/rebase.rs`) need a clean working tree, run without an editor (`-c core.editor=true`), and
  skip a commit that ends up empty after resolving. During a rebase the conflict editor names the
  sides "From <onto>" and "Your commit", because git's "ours" is the branch rebased onto.
- Line-by-line commits (`git/partial.rs`) reset the index and rebuild it: whole files are added and
  partly chosen files get a patch applied with `git apply --cached --recount`. Anything staged
  outside the app is unstaged, as GitHub Desktop does. Diff lines remember a `\r\n` ending
  (`DiffLine::crlf`) and patches put it back. A partly chosen deletion becomes a modification (its
  header is rewritten), and a rename keeps its rename headers. New files skip patches: the chosen
  lines are stored with `git hash-object -w --path`, so `core.autocrlf` and `.gitattributes`
  apply as with `git add`. Lines can't be picked in conflicts, binary or cut diffs, during a merge
  (git only commits a merge whole, a git rule rather than a missing feature) or during a rebase.
- Undo (`commits::undo_last`) is `git reset --soft HEAD~1`, or `git update-ref -d HEAD` for the
  first commit. Amend uses `git commit --amend` (with `--only` and no paths to change only the
  message). `RepoSnapshot::head_pushed` (HEAD is on a remote-tracking branch) decides whether to
  warn. A branch rewritten in the app is remembered in `RepoStore::rewritten_branches` for the
  session, and while it is both ahead and behind, the sync button offers a force push
  (`--force-with-lease`). The memory is per session: after a restart the button shows Pull again.
- Diff text is selected by whole lines, not characters: the diff only lays out visible rows.
- Image previews (`services/image_preview.rs`) decode in background tasks with the `image` crate
  (already in the tree through egui's clipboard, so only the WebP, GIF, BMP and ICO decoders were
  added). Files over 50 MB or 60 megapixels aren't decoded, and the rest are shrunk to 2048 px on
  the longer side, so a huge image can't freeze the window or use up memory. SVG, video and audio
  aren't previewed: SVG would need a renderer (resvg), and video a media stack. GIF shows its first
  frame. A renamed image is compared with its old path in Changes, but not in History.
- `show_rows` works out visible rows from the item spacing set before it is called, so a list whose
  rows touch (the diff) must set the spacing outside the closure. Setting it inside left the bottom
  of long diffs empty (fixed 2026-11-24).
- File rows are 28 px tall and their checkboxes only as big as the box, so the name sits next to it
  as in GitHub Desktop. Both are below the 40 px touch-target rule, at the owner's request
  (2026-11-24). egui sizes checkboxes, buttons and `horizontal` rows from `interact_size.y`, so
  `file_row` lowers it before laying out the row.
- Every widget has square corners (set in `theme::apply`) except checkboxes, which are drawn through
  `components::rounded`. Use it for any new checkbox.
- Cancelling sends git SIGTERM (so clone removes its half-made folder), then SIGKILL after 5 s. Only
  fetch, pull, push, publish, tag pushes and clone can be cancelled.
- `askpass::automatic_answer` gives the github.com token only to prompts for `https://github.com`
  exactly (host parsed, port 443 at most). Up to 0.1.0 it matched `//github.com` as a substring, so
  a remote like `https://github.com.example.net` would have received the token; fixed in 0.3.0 (0.2.0 was never released: its tags pointed at commits with the wrong version).
- The tag dialog reads version files from the tagged commit (`tags::project_versions`), not the
  working tree, because that's what a release workflow checks out. Added after `v0.2.0` and `v0.2.1`
  were tagged on commits whose `Cargo.toml` said 0.1.0 and 0.2.0, so both releases failed.
- GitHub Enterprise Server and `*.ghe.com` aren't supported beyond plain git: the API base, OAuth
  and `github::web_url` are fixed to github.com. Supporting them needs a per-account host, API base
  and OAuth app, and the token kept per host.
- The OAuth app (Client ID in `constants/github.rs`) has only Device Flow enabled: no client secret,
  no expiring tokens, no wildcard redirects. Its name on GitHub is "Git Desktop Linux Enhanced".
- Pull requests (`github/pulls.rs`) read the pull request, its comments, reviews, line comments and
  files (every page, up to 3,000 files) and the checks. Line comments go under their lines in
  `diff_view_with_notes`, as extra rows wrapped to the diff's width so `show_rows` still works. A
  line that already has a thread gets replies (`/comments/{id}/replies`). Merging sends the head SHA
  so GitHub refuses if new commits arrived. Draft and ready-for-review use GraphQL (the REST API
  can't change them), which reports errors with status 200, so its `errors` array is checked.
  Review comments are posted one at a time ("Add single comment"); pending reviews with several
  comments aren't offered. Merged branches aren't deleted from the app.
- A repository whose folder was moved or deleted stays in the list and shows git's error until it is
  removed from the list.
- Stashes tied to a branch use GitHub Desktop's message format (`!!GitHub_Desktop<branch>`), so
  stashes from either app show on the right branch. A branch shows its own stashes plus untagged
  ones. Unlike GitHub Desktop, a branch can hold more than one stash.
- Tags pushed to the remote aren't tracked: the app doesn't show which tags exist only locally.
- A credential prompt that nobody answers keeps that git command waiting until it's answered or
  the app closes.

## Not yet verified

- Against real GitHub (only mocked JSON and local bare remotes in tests): browser sign-in after
  approval (the device code request itself was checked against GitHub), listing organizations and
  publishing into one, listing pull requests with checks, commenting, creating a pull request,
  line comments and replies, reviews, merging, editing, and the draft switch (GraphQL). The pull
  request pages were rendered with made-up data (`tests/pull_screens.rs`).
- Network remotes on GitHub: sign-in with a real token, publishing a repository through the API, and
  pushing or pulling over HTTPS with the token. Tests cover the same git commands against local bare
  remotes, and real git getting credentials through the askpass bridge, but not GitHub itself.
- SSH prompts (key passphrase, unknown host key) through the askpass dialog.
- The keychain on the owner's machine (GNOME Keyring / KWallet). If no keychain is running, signing
  in shows an error instead of saving the token.
- The in-app updater against a real release: no release with a package has been published yet. The
  install script itself was tested (install, reinstall, uninstall) in a
  throwaway home folder.
- The GitHub Actions workflows have not run yet. The first push will show whether the Ubuntu 22.04
  runner needs more system packages.
- The window size handling was tested with unit tests and by launching the app with a corrupted
  settings file. Long-term behaviour on every compositor (GNOME, KDE, Sway, Hyprland) and with
  fractional scaling hasn't been checked by hand.
- The installed `.desktop` entry and icon haven't been checked in a desktop's app menu.
- macOS builds have not been tried.
