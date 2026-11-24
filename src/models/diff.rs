#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    /// `diff --git`, `index`, `---`, `+++` and similar lines.
    Header,
    /// `@@ -a,b +c,d @@`
    Hunk,
    Context,
    Added,
    Removed,
    /// `\ No newline at end of file`
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub lines: Vec<DiffLine>,
    pub is_binary: bool,
    pub is_truncated: bool,
}
