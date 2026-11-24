use std::path::Path;

use super::parse;
use super::runner::{GitResult, run};
use crate::models::Branch;

pub fn list(path: &Path) -> GitResult<Vec<Branch>> {
    let format = format!("--format={}", parse::branch_format());
    let output = run(path, &["for-each-ref", &format, "refs/heads", "refs/remotes"])?;
    Ok(parse::parse_branches(&output))
}

/// Switches to a local branch, or creates a tracking branch for a remote one (`origin/x` → `x`).
pub fn switch(path: &Path, branch: &Branch) -> GitResult<()> {
    if branch.is_remote {
        run(path, &["switch", "--track", &branch.name])?;
    } else {
        run(path, &["switch", &branch.name])?;
    }
    Ok(())
}

/// Creates a branch from the current HEAD and switches to it.
pub fn create(path: &Path, name: &str) -> GitResult<()> {
    run(path, &["switch", "--create", name])?;
    Ok(())
}

/// Deletes a local branch. Uses `-d`, so git refuses when the branch has unmerged commits.
pub fn delete(path: &Path, name: &str) -> GitResult<()> {
    run(path, &["branch", "--delete", name])?;
    Ok(())
}

/// Checks a name with `git check-ref-format`, so the dialog can reject it before trying.
pub fn is_valid_name(path: &Path, name: &str) -> bool {
    !name.trim().is_empty() && run(path, &["check-ref-format", "--branch", name]).is_ok()
}

/// Turns typed text into a usable branch name the way GitHub Desktop does: whitespace and
/// characters git forbids become `-`, and leading or trailing separators are dropped.
pub fn sanitize_name(input: &str) -> String {
    const FORBIDDEN: &[char] = &['~', '^', ':', '?', '*', '[', '\\', '@', '{', '}'];
    let replaced: String = input
        .trim()
        .chars()
        .map(|c| if c.is_whitespace() || c.is_control() || FORBIDDEN.contains(&c) { '-' } else { c })
        .collect();
    let mut name = replaced;
    while name.contains("..") {
        name = name.replace("..", ".");
    }
    while name.contains("--") {
        name = name.replace("--", "-");
    }
    name.trim_matches(|c| c == '-' || c == '.' || c == '/').trim_end_matches(".lock").to_owned()
}

#[cfg(test)]
mod tests {
    use super::sanitize_name;

    #[test]
    fn sanitizes_like_github_desktop() {
        assert_eq!(sanitize_name("  my new  feature "), "my-new-feature");
        assert_eq!(sanitize_name("fix: bug~1"), "fix-bug-1");
        assert_eq!(sanitize_name("a..b"), "a.b");
        assert_eq!(sanitize_name("feature/x"), "feature/x");
        assert_eq!(sanitize_name("   "), "");
    }
}
