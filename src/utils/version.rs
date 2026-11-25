//! Semantic version numbers (`1.2.3`, optionally written `v1.2.3`), used by the updater and by the
//! tag dialog's next-version suggestions.

pub type Version = (u64, u64, u64);

pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse().ok();
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

/// The highest `vX.Y.Z` / `X.Y.Z` among `tags`, and whether it used the `v` prefix.
pub fn highest(tags: &[String]) -> Option<(Version, bool)> {
    tags.iter()
        .filter_map(|tag| parse_version(tag).map(|version| (version, tag.starts_with('v'))))
        .max_by_key(|(version, _)| *version)
}

/// Next patch, minor and major tag names after the highest version tag (`v0.1.0` style when there
/// are none yet).
pub fn next_versions(tags: &[String]) -> [String; 3] {
    let ((major, minor, patch), prefixed) = highest(tags).unwrap_or(((0, 0, 0), true));
    let prefix = if prefixed { "v" } else { "" };
    let first = highest(tags).is_none();
    let patch_name =
        if first { format!("{prefix}0.1.0") } else { format!("{prefix}{major}.{minor}.{}", patch + 1) };
    [patch_name, format!("{prefix}{major}.{}.0", minor + 1), format!("{prefix}{}.0.0", major + 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("v1.10.0", "1.9.3"));
        assert!(is_newer("1.1.1", "1.1.0"));
        assert!(!is_newer("v1.1.0", "1.1.0"));
        assert!(!is_newer("1.0.9", "1.1.0"));
        assert!(!is_newer("v2.0.0-beta", "1.0.0"));
        assert!(!is_newer("nightly", "1.0.0"));
    }

    #[test]
    fn suggests_next_versions() {
        let tags = vec!["v0.9.0".to_owned(), "v0.10.2".to_owned(), "release-x".to_owned()];
        assert_eq!(next_versions(&tags), ["v0.10.3", "v0.11.0", "v1.0.0"]);
        assert_eq!(next_versions(&["1.2.3".to_owned()]), ["1.2.4", "1.3.0", "2.0.0"]);
        assert_eq!(next_versions(&[]), ["v0.1.0", "v0.1.0", "v1.0.0"]);
    }
}
