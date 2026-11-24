use std::path::Path;

/// The folder name of a repository, used as its display name.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Splits `src/ui/app.rs` into (`app.rs`, `src/ui/`) so lists can show the name prominently.
pub fn split_file_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(index) => (&path[index + 1..], &path[..=index]),
        None => (path, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_name_from_folder() {
        assert_eq!(split_file_path("src/ui/app.rs"), ("app.rs", "src/ui/"));
        assert_eq!(split_file_path("README.md"), ("README.md", ""));
    }

    #[test]
    fn display_name_is_last_component() {
        assert_eq!(display_name(Path::new("/home/me/project")), "project");
    }
}
