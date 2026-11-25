#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    /// The commit the tag points to (peeled through annotated tag objects).
    pub commit: String,
}
