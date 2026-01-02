/// Language Domain Module
/// 
/// Defines supported programming languages for TraceCraft analysis.

use std::path::Path;

/// Supported programming languages for SCIP analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
}

impl Language {
    /// Parse language from string (CLI input).
    pub fn from_str(s: &str) -> Option<Language> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "python" | "py" => Some(Language::Python),
            _ => None,
        }
    }

    /// Infer language from file extension.
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            _ => None,
        }
    }

    /// Infer language from a file path.
    pub fn from_path(path: &Path) -> Option<Language> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    /// Get the display name of the language.
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
        }
    }

    /// Get the file extensions for this language.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::Python => &["py"],
        }
    }

    /// Get the SCIP indexer command for this language.
    pub fn scip_command(&self) -> &'static str {
        match self {
            Language::Rust => "rust-analyzer",
            Language::Python => "scip-python",
        }
    }

    /// Get installation instructions for the SCIP indexer.
    pub fn install_instructions(&self) -> &'static str {
        match self {
            Language::Rust => "Install rust-analyzer: https://rust-analyzer.github.io/manual.html#installation",
            Language::Python => "Install scip-python: npm install -g @sourcegraph/scip-python",
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::Rust
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(Language::from_str("rust"), Some(Language::Rust));
        assert_eq!(Language::from_str("RUST"), Some(Language::Rust));
        assert_eq!(Language::from_str("rs"), Some(Language::Rust));
        assert_eq!(Language::from_str("python"), Some(Language::Python));
        assert_eq!(Language::from_str("py"), Some(Language::Python));
        assert_eq!(Language::from_str("java"), None);
    }

    #[test]
    fn test_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("js"), None);
    }

    #[test]
    fn test_from_path() {
        assert_eq!(Language::from_path(Path::new("src/main.rs")), Some(Language::Rust));
        assert_eq!(Language::from_path(Path::new("app.py")), Some(Language::Python));
        assert_eq!(Language::from_path(Path::new("index.js")), None);
    }

    #[test]
    fn test_scip_command() {
        assert_eq!(Language::Rust.scip_command(), "rust-analyzer");
        assert_eq!(Language::Python.scip_command(), "scip-python");
    }
}
