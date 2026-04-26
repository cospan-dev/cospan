//! Git object copy between repositories for forks.
//!
//! Given a source git URL (tangled knot, cospan node, or any git smart HTTP
//! endpoint) and a destination git URL (a cospan node's git-receive-pack),
//! this module clones all objects and refs from source to destination.
//!
//! The implementation uses libgit2 (via git2-rs) so we don't depend on the
//! git CLI being installed. The flow is:
//!
//! 1. Initialize a bare repository in a temporary directory.
//! 2. Add the source URL as a remote and fetch all refs.
//! 3. Add the destination URL as a remote and push all refs.
//! 4. Clean up the temp directory.
//!
//! Credentials are supplied by a caller-provided closure so the same
//! module works for unauthenticated public repos (Tangled knots) and
//! authenticated cospan-node destinations.

use std::path::PathBuf;

use git2::{FetchOptions, PushOptions, RemoteCallbacks, Repository};
use tempfile::TempDir;

/// Errors returned from git copy operations.
#[derive(Debug, thiserror::Error)]
pub enum GitCopyError {
    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("source has no refs to copy")]
    EmptyRepo,

    #[error("source URL is empty")]
    MissingSource,

    #[error("destination URL is empty")]
    MissingDest,
}

/// Credential provider: given a URL and requested cred types, return a
/// `git2::Cred`. Used for authenticating source fetches and destination
/// pushes independently.
pub type CredentialProvider = Box<
    dyn Fn(&str, Option<&str>, git2::CredentialType) -> Result<git2::Cred, git2::Error>
        + Send
        + Sync,
>;

/// Options for a git copy operation.
pub struct CopyOptions {
    /// Credentials for fetching from the source.
    pub source_creds: Option<CredentialProvider>,
    /// Credentials for pushing to the destination.
    pub dest_creds: Option<CredentialProvider>,
    /// Refspecs to copy. Default: all refs (`+refs/*:refs/*`).
    pub refspecs: Vec<String>,
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            source_creds: None,
            dest_creds: None,
            // Fetch all refs from source with force to allow non-FF;
            // push them to destination with matching refs.
            refspecs: vec![
                "+refs/heads/*:refs/heads/*".to_string(),
                "+refs/tags/*:refs/tags/*".to_string(),
            ],
        }
    }
}

/// Summary of a completed copy operation.
#[derive(Debug, Clone)]
pub struct CopyReport {
    /// Number of refs successfully copied.
    pub refs_copied: usize,
    /// The refs that were copied and their target OIDs.
    pub refs: Vec<(String, String)>,
}

/// Copy git objects and refs from `source_url` to `dest_url`.
///
/// Both URLs must be git smart HTTP endpoints (http:// or https://).
/// The work is performed in a temporary bare repository which is
/// deleted on return.
pub fn copy_repo(
    source_url: &str,
    dest_url: &str,
    options: CopyOptions,
) -> Result<CopyReport, GitCopyError> {
    if source_url.is_empty() {
        return Err(GitCopyError::MissingSource);
    }
    if dest_url.is_empty() {
        return Err(GitCopyError::MissingDest);
    }

    // 1. Create a temp bare repo to stage objects.
    let tmp = TempDir::new()?;
    let bare_path: PathBuf = tmp.path().join("stage.git");
    let repo = Repository::init_bare(&bare_path)?;

    // 2. Fetch from source.
    let mut source_remote = repo.remote_anonymous(source_url)?;

    let mut fetch_opts = FetchOptions::new();
    let mut fetch_callbacks = RemoteCallbacks::new();
    if let Some(provider) = options.source_creds.as_ref() {
        fetch_callbacks.credentials(move |url, user, allowed| provider(url, user, allowed));
    }
    fetch_opts.remote_callbacks(fetch_callbacks);
    fetch_opts.download_tags(git2::AutotagOption::All);

    let refspecs_str: Vec<&str> = options.refspecs.iter().map(|s| s.as_str()).collect();
    source_remote.fetch(&refspecs_str, Some(&mut fetch_opts), None)?;

    // 3. Collect the refs we actually got so we know what to push.
    let copied_refs = collect_local_refs(&repo)?;
    if copied_refs.is_empty() {
        return Err(GitCopyError::EmptyRepo);
    }

    // 4. Build push refspecs from the refs we fetched.
    // We want to push each local ref to the same name on the destination.
    let push_refspecs: Vec<String> = copied_refs
        .iter()
        .map(|(name, _)| format!("+{name}:{name}"))
        .collect();
    let push_refspec_strs: Vec<&str> = push_refspecs.iter().map(|s| s.as_str()).collect();

    // 5. Push to destination.
    let mut dest_remote = repo.remote_anonymous(dest_url)?;

    let mut push_opts = PushOptions::new();
    let mut push_callbacks = RemoteCallbacks::new();
    if let Some(provider) = options.dest_creds.as_ref() {
        push_callbacks.credentials(move |url, user, allowed| provider(url, user, allowed));
    }

    // Track per-ref push status so we fail loudly if the server rejects any.
    let push_errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let errors_clone = push_errors.clone();
    push_callbacks.push_update_reference(move |refname, status| {
        if let Some(err) = status {
            errors_clone
                .lock()
                .unwrap()
                .push(format!("{refname}: {err}"));
        }
        Ok(())
    });
    push_opts.remote_callbacks(push_callbacks);

    dest_remote.push(&push_refspec_strs, Some(&mut push_opts))?;

    let errs = push_errors.lock().unwrap();
    if !errs.is_empty() {
        return Err(GitCopyError::Git2(git2::Error::from_str(&format!(
            "destination rejected refs: {}",
            errs.join(", ")
        ))));
    }

    Ok(CopyReport {
        refs_copied: copied_refs.len(),
        refs: copied_refs,
    })
}

/// Walk the local refs database and return (name, oid) pairs for every
/// branch and tag. Used after a fetch to know what to push to the
/// destination.
fn collect_local_refs(repo: &Repository) -> Result<Vec<(String, String)>, GitCopyError> {
    let mut out = Vec::new();
    for name in repo.references()?.names() {
        let name = name?;
        if !name.starts_with("refs/heads/") && !name.starts_with("refs/tags/") {
            continue;
        }
        if let Ok(r) = repo.find_reference(name) {
            if let Some(target) = r.target() {
                out.push((name.to_string(), target.to_string()));
            } else if let Some(target) = r.symbolic_target() {
                out.push((name.to_string(), target.to_string()));
            }
        }
    }
    Ok(out)
}

/// Build a credentials provider that uses HTTP basic auth with the
/// given username and (possibly empty) password. The cospan-node
/// accepts `Authorization: Bearer did:...` via dev auth, which libgit2
/// sends as basic auth with username=did:..., password="".
pub fn basic_auth_creds(username: String, password: String) -> CredentialProvider {
    Box::new(move |_url, _user, allowed| {
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            git2::Cred::userpass_plaintext(&username, &password)
        } else {
            git2::Cred::default()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_url_errors() {
        let err = copy_repo("", "http://dest/", CopyOptions::default()).unwrap_err();
        matches!(err, GitCopyError::MissingSource);
    }

    #[test]
    fn empty_dest_url_errors() {
        let err = copy_repo("http://src/", "", CopyOptions::default()).unwrap_err();
        matches!(err, GitCopyError::MissingDest);
    }
}
