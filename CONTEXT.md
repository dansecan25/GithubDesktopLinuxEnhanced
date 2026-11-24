# Context

Rules and current state for anyone (person or agent) working on this repository. Features and setup
are in [README.md](README.md). Structure and internals are in [DOCUMENTATION.md](DOCUMENTATION.md).

## Rules

- The general rules in the owner's personal rules file apply: no commits, tags or pull requests by
  agents, no agent attribution, no secrets in tracked files.
- Commit messages start with a tag: `[ADD]`, `[UPDATE]`, `[FIX]`, `[CONFIG]`, `[REMOVE]`.
- All git access goes through `git/runner.rs`. Parsing goes in `git/parse.rs` with a unit test.
- Views only push `Action`s. Git work only runs in background tasks started by `app.rs`.
- Every user-facing string lives in `constants/text.rs`, every size or limit in `constants/`, every
  colour in `ui/theme.rs`. Reuse the shared widgets in `ui/components/` instead of styling raw egui
  widgets.
- Only use glyphs that the bundled fonts (Ubuntu and Hack) contain. egui panics on a missing glyph.
- Before saying a change is done: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, and for UI changes the screenshot test in `tests/ui_screens.rs`.
- Check new dependencies with `cargo audit` and keep the count low.

## Production setup

There is no server. Releases are built by `.github/workflows/release.yml` when a `vX.Y.Z` tag is
pushed, which creates a draft GitHub release for the owner to publish. The tag must match the version
in `Cargo.toml`. Nothing machine-specific belongs in this repository. Use a gitignored
`CONTEXT.local.md` for local notes.

## Known issues

- `cargo audit` reports four "unmaintained" warnings, all from eframe's dependency tree with no
  upgrade available yet: `ttf-parser` (fonts) and `zvariant`, `zvariant_derive`, `zbus_names` (the
  D-Bus stack used by AccessKit and the XDG folder picker). No vulnerabilities are reported.
- The folder picker blocks the window until it closes, so the window doesn't repaint while the
  picker is open.
- Diff text can't be selected or copied yet.
- Commits always include whole files. Line-by-line selection isn't implemented.
- Merge conflicts are shown with a `!` marker, but there are no conflict resolution tools. Finish the
  merge in a terminal or editor, then commit here.
- Git operations can't be cancelled, and clone shows a spinner but no progress percentage.
- A repository whose folder was moved or deleted stays in the list and shows git's error until it is
  removed from the list.
- All stashes are listed together. GitHub Desktop's one-stash-per-branch model isn't copied.

## Not yet verified

- Push, pull and fetch against a real GitHub remote. The integration tests cover clone from a local
  path and the upstream tracking it sets up, but not network remotes or credential helpers.
- The GitHub Actions workflows have not run yet. The first push will show whether the Ubuntu 22.04
  runner needs more system packages.
- The window size handling was tested with unit tests and by launching the app with a corrupted
  settings file. Long-term behaviour on every compositor (GNOME, KDE, Sway, Hyprland) and with
  fractional scaling hasn't been checked by hand.
- The installed `.desktop` entry and icon haven't been checked in a desktop's app menu.
- macOS builds have not been tried.
