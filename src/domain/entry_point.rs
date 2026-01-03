//! Entry Point Detection Module
//!
//! Detects common entry points in Rust and Python codebases.

use crate::domain::language::Language;

/// Represents a detected entry point in the codebase.
#[derive(Debug, Clone)]
pub struct EntryPoint {
    /// Unique identifier for this entry point
    pub id: String,
    /// Human-readable name (e.g., "main", "app.route('/api/users')")
    pub name: String,
    /// Type of entry point
    pub kind: EntryPointKind,
    /// File path where this entry point is defined
    pub file_path: String,
    /// Line number in the file
    pub line: Option<usize>,
}

/// Classification of entry point types
#[derive(Debug, Clone, PartialEq)]
pub enum EntryPointKind {
    // Rust
    Main,           // fn main()
    AsyncMain,      // #[tokio::main] async fn main()
    Test,           // #[test] fn test_*()
    
    // Python
    PythonMain,     // if __name__ == "__main__"
    FlaskRoute,     // @app.route(...)
    FastAPIRoute,   // @router.get(...), @app.post(...)
    DjangoView,     // def view(request): in views.py
    
    // Generic
    ExportedFunction, // pub fn / def (no decorators)
}

/// Entry point detector
pub struct EntryPointDetector {
    language: Language,
}

impl EntryPointDetector {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    /// Detect entry points from source code
    pub fn detect(&self, file_path: &str, source: &str) -> Vec<EntryPoint> {
        match self.language {
            Language::Rust => self.detect_rust(file_path, source),
            Language::Python => self.detect_python(file_path, source),
        }
    }

    fn detect_rust(&self, file_path: &str, source: &str) -> Vec<EntryPoint> {
        let mut entries = Vec::new();
        
        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            
            // fn main()
            if trimmed.starts_with("fn main(") || trimmed.starts_with("pub fn main(") {
                entries.push(EntryPoint {
                    id: format!("{}::main", file_path),
                    name: "main".to_string(),
                    kind: EntryPointKind::Main,
                    file_path: file_path.to_string(),
                    line: Some(line_num + 1),
                });
            }
            
            // #[tokio::main] - check previous line
            if (trimmed.starts_with("async fn main(") || trimmed.starts_with("pub async fn main("))
                && line_num > 0
            {
                let prev_line = source.lines().nth(line_num - 1).unwrap_or("");
                if prev_line.contains("#[tokio::main]") || prev_line.contains("#[async_std::main]") {
                    entries.push(EntryPoint {
                        id: format!("{}::async_main", file_path),
                        name: "async main".to_string(),
                        kind: EntryPointKind::AsyncMain,
                        file_path: file_path.to_string(),
                        line: Some(line_num + 1),
                    });
                }
            }
            
            // #[test]
            if trimmed.starts_with("fn test_") || trimmed.starts_with("async fn test_") {
                if line_num > 0 {
                    let prev_line = source.lines().nth(line_num - 1).unwrap_or("");
                    if prev_line.contains("#[test]") || prev_line.contains("#[tokio::test]") {
                        let fn_name = trimmed
                            .split('(')
                            .next()
                            .unwrap_or("test")
                            .replace("fn ", "")
                            .replace("async ", "")
                            .trim()
                            .to_string();
                        entries.push(EntryPoint {
                            id: format!("{}::{}", file_path, fn_name),
                            name: fn_name,
                            kind: EntryPointKind::Test,
                            file_path: file_path.to_string(),
                            line: Some(line_num + 1),
                        });
                    }
                }
            }
        }
        
        entries
    }

    fn detect_python(&self, file_path: &str, source: &str) -> Vec<EntryPoint> {
        let mut entries = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // if __name__ == "__main__":
            if trimmed.contains("__name__") && trimmed.contains("__main__") {
                entries.push(EntryPoint {
                    id: format!("{}::__main__", file_path),
                    name: "__main__".to_string(),
                    kind: EntryPointKind::PythonMain,
                    file_path: file_path.to_string(),
                    line: Some(line_num + 1),
                });
            }
            
            // Flask: @app.route(...) or @blueprint.route(...)
            if trimmed.starts_with("@") && trimmed.contains(".route(") {
                // Next line should be def ...
                if let Some(next_line) = lines.get(line_num + 1) {
                    if next_line.trim().starts_with("def ") {
                        let route_path = trimmed
                            .split("route(")
                            .nth(1)
                            .and_then(|s| s.split(')').next())
                            .unwrap_or("unknown");
                        let fn_name = next_line
                            .trim()
                            .strip_prefix("def ")
                            .and_then(|s| s.split('(').next())
                            .unwrap_or("route");
                        entries.push(EntryPoint {
                            id: format!("{}::{}", file_path, fn_name),
                            name: format!("route {}", route_path),
                            kind: EntryPointKind::FlaskRoute,
                            file_path: file_path.to_string(),
                            line: Some(line_num + 2), // Point to the def line
                        });
                    }
                }
            }
            
            // FastAPI: @router.get(...), @app.post(...), etc.
            if trimmed.starts_with("@") && 
               (trimmed.contains(".get(") || trimmed.contains(".post(") || 
                trimmed.contains(".put(") || trimmed.contains(".delete(")) 
            {
                if let Some(next_line) = lines.get(line_num + 1) {
                    if next_line.trim().starts_with("def ") || next_line.trim().starts_with("async def ") {
                        let fn_name = next_line
                            .trim()
                            .replace("async ", "")
                            .strip_prefix("def ")
                            .and_then(|s| s.split('(').next())
                            .unwrap_or("endpoint")
                            .to_string();
                        entries.push(EntryPoint {
                            id: format!("{}::{}", file_path, fn_name),
                            name: format!("API {}", fn_name),
                            kind: EntryPointKind::FastAPIRoute,
                            file_path: file_path.to_string(),
                            line: Some(line_num + 2),
                        });
                    }
                }
            }
            
            // def main():
            if trimmed == "def main():" || trimmed.starts_with("def main(") {
                entries.push(EntryPoint {
                    id: format!("{}::main", file_path),
                    name: "main".to_string(),
                    kind: EntryPointKind::PythonMain,
                    file_path: file_path.to_string(),
                    line: Some(line_num + 1),
                });
            }
        }
        
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust_main() {
        let detector = EntryPointDetector::new(Language::Rust);
        let source = r#"
fn main() {
    println!("Hello");
}
"#;
        let entries = detector.detect("src/main.rs", source);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, EntryPointKind::Main);
    }

    #[test]
    fn test_detect_python_main() {
        let detector = EntryPointDetector::new(Language::Python);
        let source = r#"
def foo():
    pass

if __name__ == "__main__":
    foo()
"#;
        let entries = detector.detect("app.py", source);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, EntryPointKind::PythonMain);
    }

    #[test]
    fn test_detect_flask_route() {
        let detector = EntryPointDetector::new(Language::Python);
        let source = r#"
@app.route('/users')
def get_users():
    return []
"#;
        let entries = detector.detect("routes.py", source);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, EntryPointKind::FlaskRoute);
        assert!(entries[0].name.contains("/users"));
    }
}
