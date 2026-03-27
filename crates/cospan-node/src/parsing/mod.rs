//! Thin dispatch layer calling panproto-parse's language parsers via the
//! unified `ParserRegistry`.
//!
//! `ParserRegistry` handles all supported languages (TypeScript, Python,
//! Rust, Java, Go, Swift, Kotlin, C#, C, C++) through a single interface.
//! Language detection is automatic based on file extension.

use std::path::Path;

use anyhow::Result;
use panproto_parse::ParserRegistry;
use panproto_schema::Schema;

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    Python,
    Rust,
    Java,
    Go,
    Swift,
    Kotlin,
    CSharp,
    RawFile,
}

impl Language {
    /// Return the panproto protocol name used by the `ParserRegistry`.
    pub fn protocol_name(self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::Python => "python",
            Language::Rust => "rust",
            Language::Java => "java",
            Language::Go => "go",
            Language::Swift => "swift",
            Language::Kotlin => "kotlin",
            Language::CSharp => "csharp",
            Language::RawFile => "raw_file",
        }
    }
}

/// Detect language from file extension.
pub fn detect_language(path: &str) -> Option<Language> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
        "py" | "pyi" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "java" => Some(Language::Java),
        "go" => Some(Language::Go),
        "swift" => Some(Language::Swift),
        "kt" | "kts" => Some(Language::Kotlin),
        "cs" => Some(Language::CSharp),
        _ => None,
    }
}

/// Parse source file into a panproto Schema by dispatching to the
/// appropriate language parser via `panproto_parse::ParserRegistry`.
///
/// The registry supports all 10+ languages through a unified interface.
/// Language detection falls back to the file path extension if no
/// explicit `Language` is provided.
pub fn parse_file(content: &[u8], language: Language) -> Result<Schema> {
    // Use a synthetic file path with the right extension so the registry
    // can verify language detection. The actual parsing uses the protocol name.
    let file_path = match language {
        Language::TypeScript => "input.ts",
        Language::Python => "input.py",
        Language::Rust => "input.rs",
        Language::Java => "Input.java",
        Language::Go => "input.go",
        Language::Swift => "input.swift",
        Language::Kotlin => "input.kt",
        Language::CSharp => "input.cs",
        Language::RawFile => "input.txt",
    };

    let registry = ParserRegistry::new();
    registry
        .parse_with_protocol(language.protocol_name(), content, file_path)
        .map_err(|e| anyhow::anyhow!("{} parse failed: {e}", language.protocol_name()))
}

/// Parse a file by auto-detecting the language from its path.
///
/// Uses `ParserRegistry::parse_file` which handles language detection
/// internally via file extension mapping.
pub fn parse_file_by_path(content: &[u8], path: &str) -> Result<Schema> {
    let registry = ParserRegistry::new();
    registry
        .parse_file(Path::new(path), content)
        .map_err(|e| anyhow::anyhow!("parse failed for {path}: {e}"))
}

/// Emit a schema back to source code bytes for the given language.
pub fn emit_file(schema: &Schema, language: Language) -> Result<Vec<u8>> {
    let registry = ParserRegistry::new();
    registry
        .emit_with_protocol(language.protocol_name(), schema)
        .map_err(|e| anyhow::anyhow!("{} emit failed: {e}", language.protocol_name()))
}
