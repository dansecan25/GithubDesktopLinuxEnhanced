//! Rewriting the last commit, as GitHub Desktop does: undo it (its changes go back to being
//! uncommitted) or amend it (the next commit replaces it).

use std::path::Path;

use super::merge;
use super::runner::{GitError, GitResult, run};
use crate::constants::text;

/// The last commit's full message, for refilling the commit form.
pub fn last_message(path: &Path) -> GitResult<String> {
    Ok(run(path, &["log", "-1", "--format=%B"])?.trim_end().to_owned())
}

/// Removes the last commit and keeps everything it changed as uncommitted changes. Returns its
/// message, so it can go back into the commit form.
pub fn undo_last(path: &Path) -> GitResult<String> {
    if merge::message(path).is_some() {
        return Err(GitError::new(text::NOT_DURING_MERGE));
    }
    let message = last_message(path)?;
    if run(path, &["rev-parse", "-q", "--verify", "HEAD~1"]).is_ok() {
        run(path, &["reset", "--soft", "HEAD~1"])?;
    } else {
        // The first commit: the branch goes back to having no commits, and the index keeps its files.
        run(path, &["update-ref", "-d", "HEAD"])?;
    }
    Ok(message)
}

/// True when the last commit is on a remote-tracking branch, so rewriting it means force pushing.
pub fn head_is_pushed(path: &Path) -> bool {
    run(path, &["for-each-ref", "--contains", "HEAD", "--count=1", "--format=%(refname)", "refs/remotes"])
        .is_ok_and(|output| !output.trim().is_empty())
}

/// Splits `Co-authored-by:` lines off the end of a message: (message, co-authors as `Name <email>`).
pub fn split_co_authors(message: &str) -> (String, Vec<String>) {
    const TRAILER: &str = "co-authored-by:";
    let mut lines: Vec<&str> = message.trim_end().lines().collect();
    let mut authors = Vec::new();
    while let Some(line) = lines.last() {
        let lower = line.to_lowercase();
        if let Some(rest) = lower.strip_prefix(TRAILER) {
            authors.push(line[line.len() - rest.len()..].trim().to_owned());
            lines.pop();
        } else {
            break;
        }
    }
    authors.reverse();
    (lines.join("\n").trim_end().to_owned(), authors)
}

#[cfg(test)]
mod tests {
    use super::split_co_authors;

    #[test]
    fn co_author_trailers_are_split_off() {
        let (message, authors) = split_co_authors(
            "Fix\n\nBody\n\nCo-authored-by: Sam <sam@example.com>\nco-authored-by: Al <a@b.c>\n",
        );
        assert_eq!(message, "Fix\n\nBody");
        assert_eq!(authors, ["Sam <sam@example.com>", "Al <a@b.c>"]);
        assert_eq!(split_co_authors("Just a summary"), ("Just a summary".to_owned(), Vec::new()));
    }
}
