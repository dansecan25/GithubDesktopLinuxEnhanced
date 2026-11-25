//! Semantic version numbers (`1.2.3`, optionally written `v1.2.3`), used by the updater and by the
//! tag dialog's next-version suggestions.

pub type Version = (u64, u64, u64);

pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse().ok();
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

/// The project version in a version file's content: `[package]` (or `[workspace.package]`)
/// `version` in Cargo.toml, `"version"` in package.json, and `version:` in pubspec.yaml without the
/// `+build` number (Flutter's `1.0.0+4` is released as `v1.0.0`).
pub fn version_in_file(file_name: &str, content: &str) -> Option<String> {
    let version = match file_name {
        "Cargo.toml" => cargo_version(content),
        "package.json" => {
            let json: serde_json::Value = serde_json::from_str(content).ok()?;
            json.get("version")?.as_str().map(str::to_owned)
        }
        "pubspec.yaml" => content
            .lines()
            .find_map(|line| line.strip_prefix("version:"))
            .map(|value| value.split('#').next().unwrap_or_default())
            .map(|value| {
                value
                    .trim()
                    .trim_matches(['"', '\''].as_ref())
                    .split('+')
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            }),
        _ => None,
    }?;
    (!version.is_empty()).then_some(version)
}

fn cargo_version(content: &str) -> Option<String> {
    let mut section = "";
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line;
            continue;
        }
        if section != "[package]" && section != "[workspace.package]" {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        if key.trim() == "version" {
            return Some(value.split('#').next().unwrap_or_default().trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Whether a tag such as `v1.2.3` names `version` (`1.2.3`).
pub fn tag_matches(tag: &str, version: &str) -> bool {
    tag.trim().trim_start_matches('v') == version.trim().trim_start_matches('v')
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

    #[test]
    fn reads_versions_from_rust_node_and_flutter_projects() {
        let cargo = "[package]\nname = \"app\"\nversion = \"0.3.0\" # bump on release\n\n[dependencies]\nserde = { version = \"1\" }\n";
        assert_eq!(version_in_file("Cargo.toml", cargo).as_deref(), Some("0.3.0"));
        let workspace = "[workspace]\nmembers = []\n[workspace.package]\nversion = \"2.0.0\"\n";
        assert_eq!(version_in_file("Cargo.toml", workspace).as_deref(), Some("2.0.0"));
        assert_eq!(version_in_file("Cargo.toml", "[dependencies]\nversion = \"9\"\n"), None);
        assert_eq!(
            version_in_file("package.json", r#"{"name": "site", "version": "1.4.2"}"#).as_deref(),
            Some("1.4.2")
        );
        assert_eq!(version_in_file("package.json", r#"{"name": "site"}"#), None);
        let pubspec = "name: player\ndescription: x\nversion: 1.0.0+4\nenvironment:\n  sdk: ^3.0.0\n";
        assert_eq!(version_in_file("pubspec.yaml", pubspec).as_deref(), Some("1.0.0"));
        assert_eq!(version_in_file("pubspec.yaml", "version: '2.1.0'\n").as_deref(), Some("2.1.0"));
    }

    #[test]
    fn tags_match_versions_with_or_without_v() {
        assert!(tag_matches("v0.3.0", "0.3.0"));
        assert!(tag_matches("1.0.0", "1.0.0"));
        assert!(!tag_matches("v0.2.0", "0.1.0"));
    }
}
