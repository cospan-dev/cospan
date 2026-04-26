//! ATProto OAuth permission scope grammar.
//!
//! Implements the scope syntax from <https://atproto.com/specs/permission>:
//!
//! - `repo:<collection>[?action=...]` — write a record in a given collection.
//! - `rpc:<lxm>?aud=<did|*>` — call an XRPC method on a specific service DID.
//! - `blob:?accept=<mime>` — upload blobs with matching MIME types.
//! - `account:<attr>[?action=read|manage]` — PDS account settings.
//! - `identity:<attr>` — handle / DID-document control.
//! - `include:<nsid>[?aud=<did|*>]` — reference a published `permission-set` Lexicon.
//!
//! Parsing is pure syntax. Resolution of `include:` against the registry of
//! permission-set lexicons happens in a separate step (see `PermissionSetRegistry`).

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

/// A single OAuth scope token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// `atproto` — required marker scope for an ATProto OAuth session.
    Atproto,
    /// `transition:generic` or `transition:<name>` — legacy broad scope.
    Transition(String),
    /// `repo:<collection>[?action=...]`
    Repo {
        collection: String,
        actions: BTreeSet<RepoAction>,
    },
    /// `rpc:<lxm>?aud=<did|*>`
    Rpc { lxm: String, aud: Aud },
    /// `blob:?accept=<mime>[&accept=<mime>...]`
    Blob { accept: Vec<String> },
    /// `account:<attr>[?action=...]`
    Account {
        attr: String,
        action: Option<AccountAction>,
    },
    /// `identity:<attr>`
    Identity { attr: String },
    /// `include:<nsid>[?aud=<did|*>]`
    Include { nsid: String, aud: Option<Aud> },
}

/// Actions permitted for a `repo:` scope. Absent set = all actions allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoAction {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountAction {
    Read,
    Manage,
}

/// Audience restriction for an `rpc:` or `include:` scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Aud {
    /// `*` — any service.
    Wildcard,
    /// A specific service DID, e.g. `did:web:api.example.com#svc_appview`.
    Did(String),
}

impl Aud {
    pub fn matches(&self, service: &str) -> bool {
        match self {
            Aud::Wildcard => true,
            Aud::Did(d) => d == service,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("empty scope")]
    Empty,
    #[error("unknown scope prefix: {0}")]
    UnknownPrefix(String),
    #[error("missing required parameter `{param}` in scope `{scope}`")]
    MissingParam { scope: String, param: &'static str },
    #[error("invalid value for `{param}` in scope `{scope}`: {value}")]
    InvalidValue {
        scope: String,
        param: &'static str,
        value: String,
    },
    #[error("malformed scope: {0}")]
    Malformed(String),
}

/// Parse a whitespace-separated scope string into a list of `Scope`s.
pub fn parse_scope_string(s: &str) -> Result<Vec<Scope>, ScopeError> {
    s.split_ascii_whitespace()
        .filter(|t| !t.is_empty())
        .map(parse_scope_token)
        .collect()
}

/// Parse a single scope token.
pub fn parse_scope_token(token: &str) -> Result<Scope, ScopeError> {
    if token.is_empty() {
        return Err(ScopeError::Empty);
    }
    if token == "atproto" {
        return Ok(Scope::Atproto);
    }
    // Split on the first ':' to get the prefix; the rest is head (positional)
    // plus optional `?query` parameters. Per spec, prefix can be followed
    // either by `:<positional>` or directly by `?<params>`.
    let (prefix, rest) = match token.find([':', '?']) {
        Some(i) => (&token[..i], &token[i..]),
        None => (token, ""),
    };

    // Strip a leading ':' for positional syntax.
    let (positional, query) = if let Some(r) = rest.strip_prefix(':') {
        match r.find('?') {
            Some(q) => (Some(&r[..q]), &r[q + 1..]),
            None => (Some(r), ""),
        }
    } else if let Some(r) = rest.strip_prefix('?') {
        (None, r)
    } else {
        (None, "")
    };

    let params = parse_query(query);

    match prefix {
        "transition" => {
            let name = positional
                .ok_or_else(|| ScopeError::Malformed(format!("transition scope: {token}")))?;
            Ok(Scope::Transition(name.to_string()))
        }
        "repo" => {
            let collection = positional
                .or_else(|| {
                    params
                        .get("collection")
                        .and_then(|v| v.first().map(String::as_str))
                })
                .ok_or(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "collection",
                })?
                .to_string();
            let mut actions = BTreeSet::new();
            if let Some(vs) = params.get("action") {
                for v in vs {
                    let a = match v.as_str() {
                        "create" => RepoAction::Create,
                        "update" => RepoAction::Update,
                        "delete" => RepoAction::Delete,
                        other => {
                            return Err(ScopeError::InvalidValue {
                                scope: token.to_string(),
                                param: "action",
                                value: other.to_string(),
                            });
                        }
                    };
                    actions.insert(a);
                }
            }
            Ok(Scope::Repo {
                collection,
                actions,
            })
        }
        "rpc" => {
            let lxm = positional
                .or_else(|| {
                    params
                        .get("lxm")
                        .and_then(|v| v.first().map(String::as_str))
                })
                .ok_or(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "lxm",
                })?
                .to_string();
            let aud = params
                .get("aud")
                .and_then(|v| v.first())
                .map(|s| parse_aud(s))
                .ok_or(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "aud",
                })?;
            Ok(Scope::Rpc { lxm, aud })
        }
        "blob" => {
            let mut accept: Vec<String> = params.get("accept").cloned().unwrap_or_default();
            if let Some(p) = positional {
                accept.insert(0, p.to_string());
            }
            if accept.is_empty() {
                return Err(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "accept",
                });
            }
            Ok(Scope::Blob { accept })
        }
        "account" => {
            let attr = positional
                .or_else(|| {
                    params
                        .get("attr")
                        .and_then(|v| v.first().map(String::as_str))
                })
                .ok_or(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "attr",
                })?
                .to_string();
            let action = match params.get("action").and_then(|v| v.first()) {
                None => None,
                Some(a) if a == "read" => Some(AccountAction::Read),
                Some(a) if a == "manage" => Some(AccountAction::Manage),
                Some(a) => {
                    return Err(ScopeError::InvalidValue {
                        scope: token.to_string(),
                        param: "action",
                        value: a.clone(),
                    });
                }
            };
            Ok(Scope::Account { attr, action })
        }
        "identity" => {
            let attr = positional
                .or_else(|| {
                    params
                        .get("attr")
                        .and_then(|v| v.first().map(String::as_str))
                })
                .ok_or(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "attr",
                })?
                .to_string();
            Ok(Scope::Identity { attr })
        }
        "include" => {
            let nsid = positional
                .or_else(|| {
                    params
                        .get("nsid")
                        .and_then(|v| v.first().map(String::as_str))
                })
                .ok_or(ScopeError::MissingParam {
                    scope: token.to_string(),
                    param: "nsid",
                })?
                .to_string();
            let aud = params
                .get("aud")
                .and_then(|v| v.first())
                .map(|s| parse_aud(s));
            Ok(Scope::Include { nsid, aud })
        }
        other => Err(ScopeError::UnknownPrefix(other.to_string())),
    }
}

fn parse_aud(s: &str) -> Aud {
    if s == "*" {
        Aud::Wildcard
    } else {
        Aud::Did(s.to_string())
    }
}

fn parse_query(q: &str) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    if q.is_empty() {
        return out;
    }
    for pair in q.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.find('=') {
            Some(i) => (&pair[..i], &pair[i + 1..]),
            None => (pair, ""),
        };
        let key = urlencoding::decode(k)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| k.to_string());
        let val = urlencoding::decode(v)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| v.to_string());
        out.entry(key).or_default().push(val);
    }
    out
}

// -- Matching -----------------------------------------------------------------

/// A concrete request that a set of granted scopes must permit.
#[derive(Debug, Clone)]
pub enum Need<'a> {
    /// Write a record in `collection` with `action`.
    Repo {
        collection: &'a str,
        action: RepoAction,
    },
    /// Call XRPC method `lxm` on service `aud`.
    Rpc { lxm: &'a str, aud: &'a str },
    /// Upload a blob of `mime` type.
    Blob { mime: &'a str },
    /// Read or manage `attr` on the PDS account.
    Account {
        attr: &'a str,
        action: AccountAction,
    },
    /// Operate on identity `attr`.
    Identity { attr: &'a str },
}

impl Scope {
    /// Does this single granted scope permit `need`?
    pub fn permits(&self, need: &Need<'_>) -> bool {
        match (self, need) {
            (Scope::Atproto | Scope::Transition(_), _) => false, // don't grant fine-grained access
            (
                Scope::Repo {
                    collection,
                    actions,
                },
                Need::Repo {
                    collection: c,
                    action: a,
                },
            ) => {
                (collection == "*" || collection == c)
                    && (actions.is_empty() || actions.contains(a))
            }
            (Scope::Rpc { lxm, aud }, Need::Rpc { lxm: l, aud: au }) => {
                (lxm == "*" || lxm == l) && aud.matches(au)
            }
            (Scope::Blob { accept }, Need::Blob { mime }) => {
                accept.iter().any(|pat| mime_matches(pat, mime))
            }
            (
                Scope::Account { attr, action },
                Need::Account {
                    attr: a,
                    action: want,
                },
            ) => {
                (attr == "*" || attr == a)
                    && match action {
                        None => true,
                        // `read` is implied by `manage`.
                        Some(AccountAction::Manage) => {
                            matches!(want, AccountAction::Read | AccountAction::Manage)
                        }
                        Some(AccountAction::Read) => matches!(want, AccountAction::Read),
                    }
            }
            (Scope::Identity { attr }, Need::Identity { attr: a }) => attr == "*" || attr == a,
            _ => false,
        }
    }
}

/// Match a MIME pattern like `video/*` against a concrete type like `video/mp4`.
fn mime_matches(pattern: &str, mime: &str) -> bool {
    if pattern == "*/*" || pattern == "*" {
        return true;
    }
    let Some((pt, ps)) = pattern.split_once('/') else {
        return false;
    };
    let Some((mt, ms)) = mime.split_once('/') else {
        return false;
    };
    (pt == "*" || pt == mt) && (ps == "*" || ps == ms)
}

// -- Permission-set registry --------------------------------------------------

/// A permission-set loaded from a Lexicon file.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    pub nsid: String,
    pub includes: Vec<String>,
    pub permissions: Vec<Scope>,
}

/// Registry of known permission-sets, indexed by NSID.
#[derive(Debug, Clone, Default)]
pub struct PermissionSetRegistry {
    sets: HashMap<String, PermissionSet>,
}

impl PermissionSetRegistry {
    pub fn new() -> Self {
        Self {
            sets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, set: PermissionSet) {
        self.sets.insert(set.nsid.clone(), set);
    }

    pub fn get(&self, nsid: &str) -> Option<&PermissionSet> {
        self.sets.get(nsid)
    }

    /// Load all permission-set lexicons from a directory tree.
    pub fn load_from_dir(path: &std::path::Path) -> anyhow::Result<Self> {
        let mut reg = Self::new();
        for entry in walkdir(path)? {
            if entry.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let bytes = std::fs::read(&entry)?;
            let v: serde_json::Value = serde_json::from_slice(&bytes)?;
            let main = v.get("defs").and_then(|d| d.get("main"));
            if main.and_then(|m| m.get("type")).and_then(|t| t.as_str()) != Some("permission-set") {
                continue;
            }
            let nsid = v
                .get("id")
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing id in {}", entry.display()))?
                .to_string();
            let main = main.unwrap();
            let includes: Vec<String> = main
                .get("includes")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let permissions: Vec<Scope> = main
                .get("permissions")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().filter_map(permission_object_to_scope).collect())
                .unwrap_or_default();
            reg.insert(PermissionSet {
                nsid,
                includes,
                permissions,
            });
        }
        Ok(reg)
    }

    /// Expand a list of scopes, resolving `include:` tokens transitively.
    /// When an `include:` has an `aud`, `inheritAud` fields on inner rpc
    /// permissions are filled from it.
    pub fn expand(&self, scopes: &[Scope]) -> Vec<Scope> {
        let mut out = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for s in scopes {
            self.expand_one(s, &mut out, &mut seen, None);
        }
        out
    }

    fn expand_one(
        &self,
        scope: &Scope,
        out: &mut Vec<Scope>,
        seen: &mut BTreeSet<String>,
        inherit_aud: Option<&Aud>,
    ) {
        match scope {
            Scope::Include { nsid, aud } => {
                if !seen.insert(nsid.clone()) {
                    return;
                }
                let Some(set) = self.sets.get(nsid) else {
                    tracing::warn!(nsid = %nsid, "unknown permission-set referenced by include:");
                    return;
                };
                let next_aud = aud.as_ref().or(inherit_aud);
                for inner in &set.permissions {
                    // Fill in inherited aud for rpc tokens lacking one.
                    let materialized = match inner {
                        Scope::Rpc { lxm, aud: a } => Scope::Rpc {
                            lxm: lxm.clone(),
                            aud: match (a, next_aud) {
                                (Aud::Wildcard, Some(a)) => a.clone(),
                                _ => a.clone(),
                            },
                        },
                        other => other.clone(),
                    };
                    self.expand_one(&materialized, out, seen, next_aud);
                }
                for inc in &set.includes {
                    self.expand_one(
                        &Scope::Include {
                            nsid: inc.clone(),
                            aud: next_aud.cloned(),
                        },
                        out,
                        seen,
                        next_aud,
                    );
                }
            }
            other => out.push(other.clone()),
        }
    }
}

fn permission_object_to_scope(p: &serde_json::Value) -> Option<Scope> {
    let resource = p.get("resource").and_then(|v| v.as_str())?;
    match resource {
        "repo" => {
            let collection = p.get("collection").and_then(|v| v.as_str())?.to_string();
            let actions = p
                .get("action")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| match v.as_str()? {
                            "create" => Some(RepoAction::Create),
                            "update" => Some(RepoAction::Update),
                            "delete" => Some(RepoAction::Delete),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Scope::Repo {
                collection,
                actions,
            })
        }
        "rpc" => {
            let lxm = p.get("lxm").and_then(|v| v.as_str())?.to_string();
            // inheritAud -> mark wildcard; will be replaced at expand time if
            // the including scope supplies one.
            let aud = if p
                .get("inheritAud")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                Aud::Wildcard
            } else {
                match p.get("aud").and_then(|v| v.as_str()) {
                    Some("*") | None => Aud::Wildcard,
                    Some(d) => Aud::Did(d.to_string()),
                }
            };
            Some(Scope::Rpc { lxm, aud })
        }
        "blob" => {
            let accept = p
                .get("accept")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(|| vec!["*/*".into()]);
            Some(Scope::Blob { accept })
        }
        "account" => {
            let attr = p.get("attr").and_then(|v| v.as_str())?.to_string();
            let action = match p.get("action").and_then(|v| v.as_str()) {
                Some("read") => Some(AccountAction::Read),
                Some("manage") => Some(AccountAction::Manage),
                _ => None,
            };
            Some(Scope::Account { attr, action })
        }
        "identity" => {
            let attr = p.get("attr").and_then(|v| v.as_str())?.to_string();
            Some(Scope::Identity { attr })
        }
        _ => None,
    }
}

fn walkdir(path: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        if !p.exists() {
            continue;
        }
        if p.is_dir() {
            for entry in std::fs::read_dir(&p)? {
                let entry = entry?;
                stack.push(entry.path());
            }
        } else {
            out.push(p);
        }
    }
    Ok(out)
}

/// Top-level check: do any of the granted scopes permit the request?
pub fn permits(granted: &[Scope], need: &Need<'_>) -> bool {
    granted.iter().any(|s| s.permits(need))
}

// -- Intent-based scope request builder --------------------------------------

/// High-level access tier a user can request at OAuth login time.
///
/// Mirrors the four Cospan permission-sets (`dev.cospan.auth.*Access`). The
/// scope string generated for each tier is what the appview sends to the PDS
/// during the authorization request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthIntent {
    /// Browse public content only — read-only RPC.
    Browse,
    /// Open issues, comment, fork, author pull requests.
    Contribute,
    /// Manage collaborators, merge PRs, triage.
    Maintain,
    /// Organization and repo administration.
    Own,
}

impl AuthIntent {
    pub fn permission_set_nsid(self) -> &'static str {
        match self {
            AuthIntent::Browse => "dev.cospan.auth.readerAccess",
            AuthIntent::Contribute => "dev.cospan.auth.contributorAccess",
            AuthIntent::Maintain => "dev.cospan.auth.maintainerAccess",
            AuthIntent::Own => "dev.cospan.auth.ownerAccess",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "browse" => Some(AuthIntent::Browse),
            "contribute" => Some(AuthIntent::Contribute),
            "maintain" => Some(AuthIntent::Maintain),
            "own" => Some(AuthIntent::Own),
            _ => None,
        }
    }
}

/// Serialize a single [`Scope`] back to its wire-format token.
fn scope_to_wire(scope: &Scope) -> Option<String> {
    match scope {
        Scope::Atproto => Some("atproto".into()),
        Scope::Transition(name) => Some(format!("transition:{name}")),
        Scope::Repo {
            collection,
            actions,
        } => {
            if actions.is_empty() {
                Some(format!("repo:{collection}"))
            } else {
                let mut s = format!("repo:{collection}?");
                let parts: Vec<String> = actions
                    .iter()
                    .map(|a| format!("action={}", match a {
                        RepoAction::Create => "create",
                        RepoAction::Update => "update",
                        RepoAction::Delete => "delete",
                    }))
                    .collect();
                s.push_str(&parts.join("&"));
                Some(s)
            }
        }
        Scope::Rpc { lxm, aud } => {
            let aud_s = match aud {
                Aud::Wildcard => "*".to_owned(),
                Aud::Did(d) => urlencoding::encode(d).into_owned(),
            };
            Some(format!("rpc:{lxm}?aud={aud_s}"))
        }
        Scope::Blob { accept } => {
            let parts: Vec<String> = accept
                .iter()
                .map(|m| format!("accept={}", urlencoding::encode(m)))
                .collect();
            Some(format!("blob?{}", parts.join("&")))
        }
        Scope::Account { attr, action } => match action {
            Some(AccountAction::Read) => Some(format!("account:{attr}?action=read")),
            Some(AccountAction::Manage) => Some(format!("account:{attr}?action=manage")),
            None => Some(format!("account:{attr}")),
        },
        Scope::Identity { attr } => Some(format!("identity:{attr}")),
        Scope::Include { .. } => {
            // `include:` references are intentionally NOT serialized into the
            // wire form: Bluesky's PDS can't resolve `dev.cospan.auth.*`
            // lexicons today (no DNS record at `_lexicon.auth.cospan.dev`),
            // so emitting `include:` collapses the consent screen to just
            // `atproto`. We instead inline the expanded `repo:` / `rpc:`
            // scopes so the PDS renders one line per concrete operation.
            None
        }
    }
}

/// Expand an intent into the flat list of inline scope tokens, using the
/// loaded permission-set registry. Falls back to a hardcoded
/// `atproto` if the registry is empty (e.g. lexicon files missing on disk).
fn expand_intent_to_inline_scopes(
    intent: AuthIntent,
    registry: &PermissionSetRegistry,
    appview_did: &str,
) -> Vec<String> {
    let aud = if appview_did.is_empty() {
        Aud::Wildcard
    } else {
        Aud::Did(appview_did.to_string())
    };
    let request = vec![Scope::Include {
        nsid: intent.permission_set_nsid().to_string(),
        aud: Some(aud),
    }];
    let expanded = registry.expand(&request);
    let mut out: Vec<String> = vec!["atproto".to_string()];
    let mut seen = std::collections::HashSet::new();
    for s in &expanded {
        if let Some(token) = scope_to_wire(s)
            && seen.insert(token.clone())
        {
            out.push(token);
        }
    }
    out
}

/// Build the scope string to send with an OAuth authorization request.
///
/// Inlined `repo:` / `rpc:` form: see [`scope_to_wire`] for why we don't
/// emit `include:` references on the wire today.
pub fn build_scope_string(
    intent: AuthIntent,
    registry: &PermissionSetRegistry,
    appview_did: &str,
) -> String {
    expand_intent_to_inline_scopes(intent, registry, appview_did).join(" ")
}

/// Build the space-separated scope list to advertise in client-metadata.json.
///
/// Declares the maximum scope an OAuth login may request (the full
/// `ownerAccess` expansion). Individual logins request a subset matching
/// their intent. Single source of truth: the lexicon files under
/// `packages/lexicons/dev/cospan/auth/`.
pub fn client_metadata_scope(registry: &PermissionSetRegistry, appview_did: &str) -> String {
    expand_intent_to_inline_scopes(AuthIntent::Own, registry, appview_did).join(" ")
}

// -- tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(s: &str) -> Scope {
        parse_scope_token(s).unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
    }

    #[test]
    fn atproto_marker() {
        assert_eq!(parse_one("atproto"), Scope::Atproto);
    }

    #[test]
    fn transition_legacy() {
        assert_eq!(
            parse_one("transition:generic"),
            Scope::Transition("generic".into())
        );
        assert_eq!(
            parse_one("transition:chat.bsky"),
            Scope::Transition("chat.bsky".into())
        );
    }

    #[test]
    fn repo_simple_and_actioned() {
        assert_eq!(
            parse_one("repo:app.bsky.feed.post"),
            Scope::Repo {
                collection: "app.bsky.feed.post".into(),
                actions: BTreeSet::new(),
            }
        );
        let Scope::Repo { actions, .. } =
            parse_one("repo:app.example.profile?action=create&action=update")
        else {
            panic!();
        };
        assert!(actions.contains(&RepoAction::Create));
        assert!(actions.contains(&RepoAction::Update));
        assert!(!actions.contains(&RepoAction::Delete));
    }

    #[test]
    fn repo_wildcard() {
        let Scope::Repo { collection, .. } = parse_one("repo:*") else {
            panic!();
        };
        assert_eq!(collection, "*");
    }

    #[test]
    fn rpc_with_aud() {
        assert_eq!(
            parse_one("rpc:app.example.moderation.createReport?aud=*"),
            Scope::Rpc {
                lxm: "app.example.moderation.createReport".into(),
                aud: Aud::Wildcard,
            }
        );
        let Scope::Rpc { aud, .. } = parse_one("rpc:foo.bar?aud=did:web:api.example.com") else {
            panic!();
        };
        assert_eq!(aud, Aud::Did("did:web:api.example.com".into()));
    }

    #[test]
    fn rpc_requires_aud() {
        assert!(parse_scope_token("rpc:foo.bar").is_err());
    }

    #[test]
    fn blob_accept() {
        let Scope::Blob { accept } = parse_one("blob?accept=video/*&accept=text/html") else {
            panic!();
        };
        assert_eq!(accept, vec!["video/*".to_string(), "text/html".into()]);
    }

    #[test]
    fn account_manage_implies_read() {
        let manage = parse_one("account:email?action=manage");
        assert!(manage.permits(&Need::Account {
            attr: "email",
            action: AccountAction::Read,
        }));
        assert!(manage.permits(&Need::Account {
            attr: "email",
            action: AccountAction::Manage,
        }));
        let read = parse_one("account:email?action=read");
        assert!(!read.permits(&Need::Account {
            attr: "email",
            action: AccountAction::Manage,
        }));
    }

    #[test]
    fn identity_wildcard() {
        let any = parse_one("identity:*");
        assert!(any.permits(&Need::Identity { attr: "handle" }));
        assert!(any.permits(&Need::Identity { attr: "signingKey" }));
    }

    #[test]
    fn include_parses() {
        assert_eq!(
            parse_one(
                "include:com.example.authBasicFeatures?aud=did:web:api.example.com%23svc_appview"
            ),
            Scope::Include {
                nsid: "com.example.authBasicFeatures".into(),
                aud: Some(Aud::Did("did:web:api.example.com#svc_appview".into())),
            }
        );
    }

    #[test]
    fn scope_string_round_trip() {
        let s = "atproto repo:app.bsky.feed.post rpc:foo.bar?aud=*";
        let scopes = parse_scope_string(s).unwrap();
        assert_eq!(scopes.len(), 3);
        assert_eq!(scopes[0], Scope::Atproto);
    }

    #[test]
    fn permits_repo_any_action_when_unrestricted() {
        let s = parse_one("repo:com.example.foo");
        assert!(s.permits(&Need::Repo {
            collection: "com.example.foo",
            action: RepoAction::Create,
        }));
        assert!(s.permits(&Need::Repo {
            collection: "com.example.foo",
            action: RepoAction::Delete,
        }));
    }

    #[test]
    fn repo_action_restriction_enforced() {
        let s = parse_one("repo:com.example.foo?action=create");
        assert!(s.permits(&Need::Repo {
            collection: "com.example.foo",
            action: RepoAction::Create,
        }));
        assert!(!s.permits(&Need::Repo {
            collection: "com.example.foo",
            action: RepoAction::Delete,
        }));
    }

    #[test]
    fn registry_expands_includes_with_inherit_aud() {
        let mut reg = PermissionSetRegistry::new();
        reg.insert(PermissionSet {
            nsid: "x.base".into(),
            includes: vec![],
            permissions: vec![Scope::Rpc {
                lxm: "foo.bar".into(),
                aud: Aud::Wildcard,
            }],
        });
        reg.insert(PermissionSet {
            nsid: "x.app".into(),
            includes: vec!["x.base".into()],
            permissions: vec![Scope::Repo {
                collection: "com.example.foo".into(),
                actions: BTreeSet::new(),
            }],
        });

        let expanded = reg.expand(&[Scope::Include {
            nsid: "x.app".into(),
            aud: Some(Aud::Did("did:web:appview".into())),
        }]);

        assert!(expanded.iter().any(|s| matches!(
            s,
            Scope::Rpc { lxm, aud: Aud::Did(d) }
                if lxm == "foo.bar" && d == "did:web:appview"
        )));
        assert!(expanded.iter().any(|s| matches!(
            s,
            Scope::Repo { collection, .. } if collection == "com.example.foo"
        )));
    }

    #[test]
    fn mime_glob_matches() {
        assert!(mime_matches("video/*", "video/mp4"));
        assert!(mime_matches("*/*", "image/png"));
        assert!(!mime_matches("video/*", "audio/mpeg"));
    }
}
