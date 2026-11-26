//! A pull request's "Files changed": its files, each file's diff with the line comments under their
//! lines, and the field for a new line comment.

use std::collections::HashMap;

use eframe::egui::{self, RichText, Ui};

use crate::constants::layout::{COMMIT_DESCRIPTION_ROWS, PANEL_PADDING, SIDEBAR_MIN_WIDTH};
use crate::constants::text;
use crate::models::{DiffLineKind, DiffSide, LineTarget, PullFile, PullRequest};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{
    LineNote, LineNotes, diff_view_with_notes, empty_state, file_row, left_clicked, loadable_status,
    muted_text, primary_button, secondary_button,
};

pub fn files(ui: &mut Ui, repo: &mut RepoStore, pull: &PullRequest, actions: &mut Vec<Action>) {
    let Loadable::Ready(detail) = &repo.pull_detail else { return loadable_status(ui, &repo.pull_detail) };
    egui::Panel::left(("pull-files", pull.number))
        .frame(egui::Frame::new().inner_margin(egui::Margin { right: PANEL_PADDING, ..Default::default() }))
        .resizable(true)
        .default_size(SIDEBAR_MIN_WIDTH)
        .min_size(SIDEBAR_MIN_WIDTH * 0.75)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("pull-file-list").auto_shrink([false, false]).show(
                ui,
                |ui| {
                    for file in &detail.files {
                        let selected = repo.selected_pull_file.as_deref() == Some(file.change.path.as_str());
                        let row = file_row(ui, &file.change, selected, None, false)
                            .on_hover_text(text::file_stats(file.additions, file.deletions));
                        if left_clicked(&row) {
                            actions.push(Action::SelectPullFile(file.change.path.clone()));
                        }
                    }
                },
            );
        });
    let Some(file) = repo.selected_pull_file.as_deref().and_then(|path| detail.file(path)) else {
        return empty_state(ui, text::SELECT_PULL_FILE, "");
    };
    let file = file.clone();
    let threads = threads(detail.comments_on(&file.change.path), &file);

    if let Some((target, body)) =
        repo.line_comment.as_mut().filter(|(target, _)| target.path == file.change.path)
    {
        egui::Panel::bottom(("line-comment", pull.number))
            .frame(egui::Frame::new().inner_margin(egui::Margin::same(PANEL_PADDING)))
            .show(ui, |ui| composer(ui, target, body, repo.pull_sending, actions));
    }
    egui::CentralPanel::no_frame().show(ui, |ui| {
        if file.diff_missing {
            return empty_state(ui, text::NO_PULL_DIFF, "");
        }
        ui.label(muted_text(ui, text::LINE_COMMENT_HINT));
        let notes = LineNotes { notes: &threads.notes };
        let id = format!("pull-diff-{}-{}", pull.number, file.change.path);
        if let Some(index) = diff_view_with_notes(ui, &file.diff, &id, &notes)
            && let Some(target) = target_for(&file, index, threads.roots.get(&index).copied())
        {
            actions.push(Action::StartLineComment(target));
        }
    });
}

fn composer(ui: &mut Ui, target: &LineTarget, body: &mut String, sending: bool, actions: &mut Vec<Action>) {
    ui.label(RichText::new(text::line_comment_on(&target.path, target.line)).strong());
    let hint = if target.reply_to.is_some() { text::REPLY_FIELD } else { text::LINE_COMMENT_FIELD };
    let field = ui.add(
        egui::TextEdit::multiline(body)
            .hint_text(hint)
            .desired_rows(COMMIT_DESCRIPTION_ROWS)
            .desired_width(f32::INFINITY),
    );
    if body.is_empty() && !field.has_focus() {
        field.request_focus();
    }
    ui.horizontal(|ui| {
        let label = if target.reply_to.is_some() { text::REPLY_BUTTON } else { text::ADD_LINE_COMMENT };
        if primary_button(ui, label, !body.trim().is_empty() && !sending).clicked() {
            actions.push(Action::PostLineComment);
        }
        if secondary_button(ui, text::CANCEL, true).clicked() {
            actions.push(Action::CancelLineComment);
        }
    });
}

/// The line comments of one file, placed under their lines.
struct Threads {
    notes: HashMap<usize, Vec<LineNote>>,
    /// For each line with comments, the first comment of its thread (what a reply answers).
    roots: HashMap<usize, u64>,
}

fn threads<'a>(comments: impl Iterator<Item = &'a crate::models::ReviewComment>, file: &PullFile) -> Threads {
    let mut threads = Threads { notes: HashMap::new(), roots: HashMap::new() };
    for comment in comments {
        let Some(index) = comment.line.and_then(|line| line_index(file, line, comment.side)) else {
            continue;
        };
        threads.roots.entry(index).or_insert(comment.reply_to.unwrap_or(comment.id));
        threads.notes.entry(index).or_default().push(LineNote {
            header: format!("{} · {}", comment.author, super::pulls::date(&comment.created_at)),
            body: comment.body.clone(),
        });
    }
    threads
}

/// Where `line` of one side is in the diff. A removed line is matched on the old side before a
/// context line with the same number.
fn line_index(file: &PullFile, line: u32, side: DiffSide) -> Option<usize> {
    let lines = &file.diff.lines;
    match side {
        DiffSide::Right => lines.iter().position(|l| l.new_number == Some(line)),
        DiffSide::Left => lines
            .iter()
            .position(|l| l.kind == DiffLineKind::Removed && l.old_number == Some(line))
            .or_else(|| lines.iter().position(|l| l.old_number == Some(line))),
    }
}

/// The comment target for a clicked diff line: removed lines are on the old side, the rest on the
/// new side, as GitHub numbers them.
fn target_for(file: &PullFile, index: usize, reply_to: Option<u64>) -> Option<LineTarget> {
    let line = file.diff.lines.get(index)?;
    let (number, side) = match line.kind {
        DiffLineKind::Removed => (line.old_number?, DiffSide::Left),
        DiffLineKind::Added | DiffLineKind::Context => (line.new_number?, DiffSide::Right),
        _ => return None,
    };
    Some(LineTarget { path: file.change.path.clone(), line: number, side, reply_to })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::parse::parse_diff;
    use crate::models::{ChangeKind, FileChange, ReviewComment};

    fn file() -> PullFile {
        PullFile {
            change: FileChange { path: "a.rs".into(), original_path: None, kind: ChangeKind::Modified },
            additions: 1,
            deletions: 1,
            diff: parse_diff("@@ -1,2 +1,2 @@\n keep\n-old\n+new"),
            diff_missing: false,
        }
    }

    #[test]
    fn comments_go_under_their_line_and_replies_answer_the_thread() {
        let file = file();
        let comment = |id, reply_to, line, side| ReviewComment {
            id,
            reply_to,
            path: "a.rs".into(),
            line,
            side,
            author: "sam".into(),
            body: "hm".into(),
            created_at: "2026-11-24T10:00:00Z".into(),
        };
        let comments = [
            comment(1, None, Some(2), DiffSide::Right),
            comment(2, Some(1), Some(2), DiffSide::Right),
            comment(3, None, Some(2), DiffSide::Left),
            comment(4, None, None, DiffSide::Right),
        ];
        let threads = threads(comments.iter(), &file);
        // Lines: 0 hunk, 1 " keep", 2 "-old", 3 "+new".
        assert_eq!(threads.notes[&3].len(), 2);
        assert_eq!(threads.notes[&2].len(), 1);
        assert_eq!(threads.roots[&3], 1);
        assert_eq!(threads.notes.len(), 2, "the outdated comment has no line");

        let target = target_for(&file, 2, None).unwrap();
        assert_eq!((target.line, target.side), (2, DiffSide::Left));
        assert_eq!(target_for(&file, 1, Some(9)).unwrap().reply_to, Some(9));
        assert!(target_for(&file, 0, None).is_none());
    }
}
