#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub summary: String,
    pub body: String,
    pub author: String,
    pub author_email: String,
    pub relative_date: String,
}

/// Someone credited on a commit with a `Co-authored-by:` line, which GitHub shows as an author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoAuthor {
    pub name: String,
    pub email: String,
}

impl CoAuthor {
    /// Reads `Name <email>`.
    pub fn parse(text: &str) -> Option<Self> {
        let (name, rest) = text.trim().split_once('<')?;
        let email = rest.strip_suffix('>')?.trim();
        let name = name.trim();
        (!name.is_empty() && email.contains('@') && !email.contains(char::is_whitespace))
            .then(|| Self { name: name.to_owned(), email: email.to_owned() })
    }

    pub fn trailer(&self) -> String {
        format!("Co-authored-by: {} <{}>", self.name, self.email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_email() {
        let author = CoAuthor::parse(" Sam Chen <sam@example.com> ").unwrap();
        assert_eq!(author.trailer(), "Co-authored-by: Sam Chen <sam@example.com>");
        assert!(CoAuthor::parse("Sam").is_none());
        assert!(CoAuthor::parse("<sam@example.com>").is_none());
        assert!(CoAuthor::parse("Sam <not an email>").is_none());
    }
}
