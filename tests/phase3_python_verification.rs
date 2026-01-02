/// Phase 3 Python Support Verification Tests
/// 
/// Tests the multi-language SCIP infrastructure without requiring
/// external tools (scip-python, rust-analyzer) to be installed.

use mr_hedgehog::domain::language::Language;
use mr_hedgehog::infrastructure::scip_runner::{build_command_spec, ScipCommandSpec};

/// Verify that the Language enum correctly parses string inputs.
#[test]
fn test_language_parsing_rust_variants() {
    assert_eq!(Language::from_str("rust"), Some(Language::Rust));
    assert_eq!(Language::from_str("Rust"), Some(Language::Rust));
    assert_eq!(Language::from_str("RUST"), Some(Language::Rust));
    assert_eq!(Language::from_str("rs"), Some(Language::Rust));
}

#[test]
fn test_language_parsing_python_variants() {
    assert_eq!(Language::from_str("python"), Some(Language::Python));
    assert_eq!(Language::from_str("Python"), Some(Language::Python));
    assert_eq!(Language::from_str("PYTHON"), Some(Language::Python));
    assert_eq!(Language::from_str("py"), Some(Language::Python));
}

#[test]
fn test_language_parsing_invalid() {
    assert_eq!(Language::from_str("java"), None);
    assert_eq!(Language::from_str("javascript"), None);
    assert_eq!(Language::from_str(""), None);
}

/// Verify that the correct SCIP command is selected for Rust.
#[test]
fn test_rust_command_spec() {
    let spec = build_command_spec(Language::Rust);
    
    assert_eq!(spec.program, "rust-analyzer", 
        "Rust should use rust-analyzer as the SCIP indexer");
    assert!(spec.args.contains(&"scip".to_string()), 
        "rust-analyzer command should include 'scip' subcommand");
    assert!(spec.args.contains(&"--output".to_string()), 
        "Command should include --output flag");
}

/// Verify that the correct SCIP command is selected for Python.
#[test]
fn test_python_command_spec() {
    let spec = build_command_spec(Language::Python);
    
    assert_eq!(spec.program, "scip-python", 
        "Python should use scip-python as the SCIP indexer");
    assert!(spec.args.contains(&"index".to_string()), 
        "scip-python command should include 'index' subcommand");
    assert!(spec.args.contains(&"--output".to_string()), 
        "Command should include --output flag");
}

/// Verify that Rust and Python produce different command specs.
#[test]
fn test_language_command_differentiation() {
    let rust_spec = build_command_spec(Language::Rust);
    let python_spec = build_command_spec(Language::Python);
    
    assert_ne!(rust_spec.program, python_spec.program,
        "Different languages should use different indexer programs");
    assert_ne!(rust_spec.args[0], python_spec.args[0],
        "Different languages may use different subcommands");
}

/// Verify Language enum properties.
#[test]
fn test_language_properties() {
    // Rust properties
    assert_eq!(Language::Rust.name(), "Rust");
    assert_eq!(Language::Rust.scip_command(), "rust-analyzer");
    assert!(Language::Rust.extensions().contains(&"rs"));
    
    // Python properties
    assert_eq!(Language::Python.name(), "Python");
    assert_eq!(Language::Python.scip_command(), "scip-python");
    assert!(Language::Python.extensions().contains(&"py"));
}

/// Verify install instructions are provided.
#[test]
fn test_install_instructions() {
    let rust_instructions = Language::Rust.install_instructions();
    assert!(rust_instructions.contains("rust-analyzer"), 
        "Rust instructions should mention rust-analyzer");
    
    let python_instructions = Language::Python.install_instructions();
    assert!(python_instructions.contains("scip-python"), 
        "Python instructions should mention scip-python");
    assert!(python_instructions.contains("npm"), 
        "Python instructions should mention npm installation");
}

/// Verify file extension inference.
#[test]
fn test_extension_inference() {
    use std::path::Path;
    
    assert_eq!(Language::from_path(Path::new("main.rs")), Some(Language::Rust));
    assert_eq!(Language::from_path(Path::new("src/lib.rs")), Some(Language::Rust));
    assert_eq!(Language::from_path(Path::new("app.py")), Some(Language::Python));
    assert_eq!(Language::from_path(Path::new("package/module.py")), Some(Language::Python));
    assert_eq!(Language::from_path(Path::new("index.js")), None);
    assert_eq!(Language::from_path(Path::new("Makefile")), None);
}

/// Verify default language is Rust.
#[test]
fn test_default_language() {
    assert_eq!(Language::default(), Language::Rust);
}

/// Verify command spec output format matches expected structure.
#[test]
fn test_command_spec_structure() {
    let spec = build_command_spec(Language::Rust);
    
    // Should have at least: subcommand, ".", "--output", "index.scip"
    assert!(spec.args.len() >= 4, 
        "Command should have at least 4 arguments");
    assert!(spec.args.contains(&".".to_string()), 
        "Command should target current directory");
    assert!(spec.args.contains(&"index.scip".to_string()), 
        "Output file should be index.scip");
}
