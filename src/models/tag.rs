#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    /// The commit the tag points to (peeled through annotated tag objects).
    pub commit: String,
}

/// A version written in a project file (`Cargo.toml`, `package.json`, `pubspec.yaml`) at a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectVersion {
    /// Path in the repository, like `free_music_player/pubspec.yaml`.
    pub path: String,
    pub version: String,
}
