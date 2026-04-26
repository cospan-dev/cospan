//! Codegen tests: verify the lexicon -> SQL/Rust/TypeScript pipeline.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("packages").exists() {
            return dir;
        }
        if !dir.pop() {
            panic!("could not find workspace root");
        }
    }
}

fn discover_lexicons(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(dir, &mut files);
    files.sort();
    files
}

fn walk_dir(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, files);
        } else if path.extension().is_some_and(|e| e == "json") {
            files.push(path);
        }
    }
}

fn run_codegen() -> (String, String, String) {
    let root = workspace_root();
    let lexicon_files = discover_lexicons(&root.join("packages/lexicons"));
    assert!(!lexicon_files.is_empty());

    let mut all_rust = String::new();
    let mut all_sql = String::new();
    let mut all_ts = String::new();

    for path in &lexicon_files {
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let nsid = json.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let Ok(schema) = panproto_protocols::web_document::atproto::parse_lexicon(&json) else {
            continue;
        };

        if let Ok(s) = cospan_codegen::morphism::atproto_to_rust(&schema, nsid)
            && let Ok(c) = cospan_codegen::morphism::emit_rust_types(&s)
        {
            all_rust.push_str(&c);
        }
        if let Ok(s) = cospan_codegen::morphism::atproto_to_sql(&schema, nsid)
            && let Ok(c) = cospan_codegen::morphism::emit_ddl(&s)
        {
            all_sql.push_str(&c);
        }
        if let Ok(s) = cospan_codegen::morphism::atproto_to_typescript(&schema, nsid)
            && let Ok(c) = cospan_codegen::morphism::emit_ts_types(&s)
        {
            all_ts.push_str(&c);
        }
    }
    (all_rust, all_sql, all_ts)
}

#[test]
fn sql_output_contains_tables() {
    let (_, sql, _) = run_codegen();
    assert!(!sql.is_empty());
    assert!(sql.matches("CREATE TABLE").count() >= 15);
}

#[test]
fn rust_output_contains_structs() {
    let (rust, _, _) = run_codegen();
    assert!(!rust.is_empty());
    assert!(rust.matches("struct ").count() >= 10);
}

#[test]
fn typescript_output_contains_interfaces() {
    let (_, _, ts) = run_codegen();
    assert!(!ts.is_empty());
    assert!(ts.matches("interface ").count() >= 10);
}

#[test]
fn all_lexicon_files_found() {
    let files = discover_lexicons(&workspace_root().join("packages/lexicons"));
    // 20 dev.cospan.* + 74 sh.tangled.* lexicon files
    assert!(
        files.len() >= 20,
        "expected at least 20 lexicon files, found {}",
        files.len()
    );
}

#[test]
fn tangled_issue_transform_decomposes_repo_uri() {
    let root = workspace_root();
    let lexicons_dir = root.join("packages/lexicons");
    let morphisms = cospan_codegen::tangled_interop::compile_all_morphisms(&lexicons_dir).unwrap();

    let issue_morphism = morphisms
        .iter()
        .find(|m| m.tangled_nsid == "sh.tangled.repo.issue")
        .expect("should have sh.tangled.repo.issue morphism");

    // Simulate a Tangled issue record
    let record = serde_json::json!({
        "$type": "sh.tangled.repo.issue",
        "repo": "at://did:plc:abc123/sh.tangled.repo/myrepo",
        "title": "Test issue",
        "body": "Test body",
        "createdAt": "2026-01-01T00:00:00Z"
    });

    // Parse against the Tangled schema (body vertex, not record vertex)
    let body_vertex = format!("{}:body", issue_morphism.tangled_nsid);
    let instance =
        panproto_inst::parse::parse_json(&issue_morphism.tangled_schema, &body_vertex, &record)
            .expect("should parse Tangled issue");

    // Apply morphism (includes field transforms)
    let lifted = panproto_mig::lift_wtype_sigma(
        &issue_morphism.compiled,
        &issue_morphism.cospan_schema,
        &instance,
    )
    .expect("should lift");

    // Emit JSON
    let output = panproto_inst::parse::to_json(&issue_morphism.cospan_schema, &lifted);

    eprintln!(
        "Transform output: {}",
        serde_json::to_string_pretty(&output).unwrap()
    );

    // The repo AT-URI should be decomposed into repoDid + repoName
    assert!(
        output.get("repoDid").is_some(),
        "should have repoDid field, got keys: {:?}",
        output.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );
    assert_eq!(output["repoDid"], "did:plc:abc123");
    assert_eq!(output["repoName"], "myrepo");
}
