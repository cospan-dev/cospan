//! Generate `apps/web/src/lib/generated/scopes.ts` from the
//! `dev.cospan.auth.*Access` permission-set lexicons.
//!
//! Reads every `permission-set` lexicon under
//! `packages/lexicons/dev/cospan/auth/`, transitively expands `includes`,
//! and emits a TypeScript module with one constant per permission-set
//! holding the flat array of inlined `repo:` / `rpc:` scopes that the
//! browser OAuth client requests.
//!
//! Single source of truth: the JSON Lexicon files. The frontend never
//! hand-mirrors scope lists; flipping `include:` resolution back on
//! later (once DNS lexicon resolution lands) is one
//! `expand_to_inline` toggle away.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

/// One parsed `permission-set` lexicon.
struct PermSet {
    nsid: String,
    title: Option<String>,
    detail: Option<String>,
    includes: Vec<String>,
    /// Flat scope tokens defined directly on this set (no include expansion).
    scopes: Vec<String>,
}

/// Walk `lexicons_dir/dev/cospan/auth/` and parse every `permission-set`.
fn load_permission_sets(lexicons_dir: &Path) -> Result<BTreeMap<String, PermSet>> {
    let mut out = BTreeMap::new();
    let auth_dir = lexicons_dir.join("dev").join("cospan").join("auth");
    if !auth_dir.is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&auth_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let json: Value = serde_json::from_slice(&std::fs::read(&path)?)
            .with_context(|| format!("parsing {}", path.display()))?;
        let main = json.get("defs").and_then(|d| d.get("main"));
        if main.and_then(|m| m.get("type")).and_then(|t| t.as_str()) != Some("permission-set") {
            continue;
        }
        let nsid = json
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing id in {}", path.display()))?
            .to_string();
        let main = main.unwrap();
        let title = main
            .get("title")
            .and_then(|v| v.as_str())
            .map(String::from);
        let detail = main
            .get("detail")
            .and_then(|v| v.as_str())
            .map(String::from);
        let includes: Vec<String> = main
            .get("includes")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let scopes: Vec<String> = main
            .get("permissions")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(permission_to_wire).collect())
            .unwrap_or_default();
        out.insert(
            nsid.clone(),
            PermSet {
                nsid,
                title,
                detail,
                includes,
                scopes,
            },
        );
    }
    Ok(out)
}

/// Render a `permission` JSON object as its inline wire-format token.
///
/// Matches `crates/cospan-appview/src/auth/scope.rs::scope_to_wire` in
/// the runtime path; both must agree so codegen output is what the
/// appview's client-metadata.json advertises.
fn permission_to_wire(p: &Value) -> Option<String> {
    let resource = p.get("resource").and_then(|v| v.as_str())?;
    match resource {
        "repo" => {
            let collection = p.get("collection").and_then(|v| v.as_str())?;
            let actions: Vec<&str> = p
                .get("action")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if actions.is_empty() {
                Some(format!("repo:{collection}"))
            } else {
                let qs: Vec<String> = actions.iter().map(|a| format!("action={a}")).collect();
                Some(format!("repo:{collection}?{}", qs.join("&")))
            }
        }
        "rpc" => {
            let lxm = p.get("lxm").and_then(|v| v.as_str())?;
            // We always emit `aud=*` here; the appview substitutes its own
            // service DID at runtime when validating granted scopes. For
            // the consent screen, `aud=*` is what the PDS sees, which is
            // fine — `*` renders as "any service" but the lxm itself names
            // the specific RPC method.
            Some(format!("rpc:{lxm}?aud=*"))
        }
        "blob" => {
            let accept: Vec<&str> = p
                .get("accept")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if accept.is_empty() {
                Some("blob?accept=*/*".into())
            } else {
                let qs: Vec<String> = accept.iter().map(|m| format!("accept={m}")).collect();
                Some(format!("blob?{}", qs.join("&")))
            }
        }
        "account" => {
            let attr = p.get("attr").and_then(|v| v.as_str())?;
            match p.get("action").and_then(|v| v.as_str()) {
                Some(a) => Some(format!("account:{attr}?action={a}")),
                None => Some(format!("account:{attr}")),
            }
        }
        "identity" => {
            let attr = p.get("attr").and_then(|v| v.as_str())?;
            Some(format!("identity:{attr}"))
        }
        _ => None,
    }
}

/// Recursively expand `start_nsid` against `sets`, returning the unique
/// flat list of inline scope tokens. Always prepends `atproto`.
fn expand(sets: &BTreeMap<String, PermSet>, start_nsid: &str) -> Vec<String> {
    let mut out = vec!["atproto".to_string()];
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    walk(sets, start_nsid, &mut out, &mut seen, &mut visited);
    out
}

fn walk(
    sets: &BTreeMap<String, PermSet>,
    nsid: &str,
    out: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) {
    if !visited.insert(nsid.to_string()) {
        return;
    }
    let Some(set) = sets.get(nsid) else {
        eprintln!("  warn: emit_scopes: unknown include reference: {nsid}");
        return;
    };
    for inc in &set.includes {
        walk(sets, inc, out, seen, visited);
    }
    for tok in &set.scopes {
        if seen.insert(tok.clone()) {
            out.push(tok.clone());
        }
    }
}

/// Map from cospan `dev.cospan.auth.<x>Access` NSID to the `AuthIntent`
/// constant name used in the frontend. Keep in sync with
/// `crates/cospan-appview/src/auth/scope.rs::AuthIntent`.
fn intent_for_nsid(nsid: &str) -> Option<&'static str> {
    match nsid {
        "dev.cospan.auth.readerAccess" => Some("browse"),
        "dev.cospan.auth.contributorAccess" => Some("contribute"),
        "dev.cospan.auth.maintainerAccess" => Some("maintain"),
        "dev.cospan.auth.ownerAccess" => Some("own"),
        "dev.cospan.auth.pushAccess" => Some("push"),
        _ => None,
    }
}

/// Render the generated TypeScript module body.
pub fn emit_scopes_ts(lexicons_dir: &Path) -> Result<String> {
    let sets = load_permission_sets(lexicons_dir)?;

    let mut buf = String::new();
    buf.push_str("// Generated by cospan-codegen from packages/lexicons/dev/cospan/auth/.\n");
    buf.push_str("// Do not edit manually. Source of truth is the permission-set lexicons.\n\n");

    buf.push_str("export type AuthIntent = 'browse' | 'contribute' | 'maintain' | 'own';\n\n");

    // Per-intent flat scope arrays, plus rich metadata (title/detail) for UI.
    buf.push_str("export interface PermissionSetMeta {\n");
    buf.push_str("  readonly nsid: string;\n");
    buf.push_str("  readonly title?: string;\n");
    buf.push_str("  readonly detail?: string;\n");
    buf.push_str("  /** Flat inline scope tokens including `atproto` and every transitively-included repo:/rpc: scope. */\n");
    buf.push_str("  readonly scopes: readonly string[];\n");
    buf.push_str("}\n\n");

    let mut intent_to_nsid: BTreeMap<&'static str, &str> = BTreeMap::new();
    for nsid in sets.keys() {
        if let Some(intent) = intent_for_nsid(nsid) {
            intent_to_nsid.insert(intent, nsid);
        }
    }

    buf.push_str("export const PERMISSION_SETS: Readonly<Record<AuthIntent, PermissionSetMeta>> = {\n");
    for intent in ["browse", "contribute", "maintain", "own"] {
        let nsid = match intent_to_nsid.get(intent) {
            Some(n) => *n,
            None => {
                buf.push_str(&format!(
                    "  {intent}: {{ nsid: 'dev.cospan.auth.MISSING_{intent}Access', scopes: ['atproto'] }},\n"
                ));
                continue;
            }
        };
        let set = &sets[nsid];
        let scopes = expand(&sets, nsid);
        buf.push_str(&format!("  {intent}: {{\n"));
        buf.push_str(&format!("    nsid: '{}',\n", set.nsid));
        if let Some(t) = &set.title {
            buf.push_str(&format!("    title: {},\n", json_string(t)));
        }
        if let Some(d) = &set.detail {
            buf.push_str(&format!("    detail: {},\n", json_string(d)));
        }
        buf.push_str("    scopes: [\n");
        for s in scopes {
            buf.push_str(&format!("      {},\n", json_string(&s)));
        }
        buf.push_str("    ] as const,\n");
        buf.push_str("  },\n");
    }
    buf.push_str("};\n\n");

    // Build the scope string the browser OAuth client passes to authorize().
    buf.push_str(
        "/**\n\
 * Build the inline scope string for an OAuth login request.\n\
 *\n\
 * Inlined `repo:` / `rpc:` form (not `include:`): Bluesky's PDS can't yet\n\
 * resolve `dev.cospan.auth.*` lexicons (no DNS TXT record at\n\
 * `_lexicon.auth.cospan.dev`), so emitting `include:` collapses the\n\
 * consent screen to just `atproto`. Flat tokens render as one consent\n\
 * line per concrete operation. Flip back to `include:` once lexicon\n\
 * resolution is in place.\n\
 */\n",
    );
    buf.push_str("export function buildScopeString(intent: AuthIntent = 'contribute'): string {\n");
    buf.push_str("  return PERMISSION_SETS[intent].scopes.join(' ');\n");
    buf.push_str("}\n\n");

    buf.push_str(
        "/**\n\
 * Maximum scope advertised in `client-metadata.json`. Always the union\n\
 * of every intent's scopes (= `own`). An OAuth login may request any\n\
 * subset; the PDS rejects anything broader.\n\
 */\n",
    );
    buf.push_str("export const CLIENT_METADATA_SCOPE: string = PERMISSION_SETS.own.scopes.join(' ');\n");

    Ok(buf)
}

/// Cheap JSON-string escaper for the codegen output. Quotes the value
/// and escapes `\\`, `'`, newline, carriage return, tab.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// Emit the file to the given path (creates parent dir if needed).
#[allow(dead_code)]
pub fn write_scopes_ts(out_path: &PathBuf, lexicons_dir: &Path) -> Result<()> {
    let body = emit_scopes_ts(lexicons_dir)?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out_path, body)?;
    Ok(())
}
