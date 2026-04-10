//! Spawn in-process cospan-node instances for git source/destination in tests.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use tempfile::TempDir;
use tokio::sync::Mutex;

static TRACING: OnceLock<()> = OnceLock::new();

fn init_test_tracing() {
    TRACING.get_or_init(|| {
        use tracing_subscriber::{EnvFilter, fmt};
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("debug,hyper=info,tower=info,sqlx=warn,h2=info,rustls=info"));
        let _ = fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

/// A running cospan-node instance backed by a temp directory.
pub struct TestNode {
    pub url: String,
    pub _tmp: TempDir,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestNode {
    /// Start a new node on an ephemeral port.
    pub async fn spawn() -> Self {
        // Enable dev auth so tests can use raw `Authorization: Bearer did:...` tokens.
        // SAFETY: tests run single-threaded by default under the axum runtime;
        // this env var just tells cospan-node to accept dev auth.
        unsafe {
            std::env::set_var("COSPAN_DEV_AUTH", "1");
        }
        init_test_tracing();

        let tmp = TempDir::new().unwrap();
        let config = cospan_node::config::NodeConfig {
            did: "did:plc:testnode".to_string(),
            listen: "127.0.0.1:0".to_string(),
            data_dir: tmp.path().to_path_buf(),
            validation: cospan_node::config::ValidationConfig::default(),
            auth: cospan_node::config::AuthConfig {
                allowed_dids: vec![], // allow all
            },
        };
        let repos_dir = config.repos_dir();
        tokio::fs::create_dir_all(&repos_dir).await.unwrap();

        let state = Arc::new(cospan_node::state::NodeState {
            config: config.clone(),
            store: Arc::new(Mutex::new(cospan_node::store::RepoManager::new(
                repos_dir,
            ))),
            validator: Arc::new(cospan_node::validation::ValidationPipeline::new(
                &config.validation,
            )),
            authz: Arc::new(cospan_node::auth::SimpleAuthz::new(vec![])),
            pds_client: Arc::new(NoopPdsClient),
        });

        let app = cospan_node::router::build(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });

        Self {
            url: format!("http://{addr}"),
            _tmp: tmp,
            shutdown: Some(tx),
        }
    }

    /// The base URL for git smart HTTP operations on a given repo.
    /// e.g. `http://127.0.0.1:PORT/did:plc:foo/myrepo`
    pub fn git_url(&self, did: &str, repo: &str) -> String {
        format!("{}/{}/{}", self.url, did, repo)
    }
}

impl Drop for TestNode {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

struct NoopPdsClient;

#[async_trait]
impl cospan_node::pds_client::PdsClient for NoopPdsClient {
    async fn create_record(
        &self,
        did: &str,
        collection: &str,
        _record: &serde_json::Value,
    ) -> Result<String, cospan_node::error::NodeError> {
        Ok(format!("at://{did}/{collection}/test-rkey"))
    }
}

/// Create a git repository at a temp location with a single commit,
/// then push it to the given cospan-node URL. Returns the commit OID.
///
/// Performs all git2 work on a blocking thread so it's safe to call from
/// any tokio runtime flavor.
pub async fn seed_git_repo(
    node_git_url: &str,
    files: &[(&str, &str)],
) -> (String, TempDir) {
    let url = node_git_url.to_string();
    let owned_files: Vec<(String, String)> = files
        .iter()
        .map(|(p, c)| (p.to_string(), c.to_string()))
        .collect();
    tokio::task::spawn_blocking(move || seed_git_repo_blocking(&url, &owned_files))
        .await
        .expect("seed_git_repo task panicked")
}

fn seed_git_repo_blocking(
    node_git_url: &str,
    files: &[(String, String)],
) -> (String, TempDir) {
    use git2::{IndexAddOption, Repository, Signature};

    let tmp = TempDir::new().unwrap();
    let repo_dir: PathBuf = tmp.path().join("src-repo");
    std::fs::create_dir_all(&repo_dir).unwrap();

    let repo = Repository::init(&repo_dir).unwrap();

    for (path, content) in files {
        let full = repo_dir.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content).unwrap();
    }

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let sig = Signature::now("Test", "test@example.com").unwrap();
    let commit_id = repo
        .commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

    // Make sure the default branch is `main` for consistency.
    let head_ref = repo.head().unwrap();
    if let Some(name) = head_ref.shorthand() {
        if name != "main" {
            repo.branch("main", &repo.find_commit(commit_id).unwrap(), false)
                .unwrap();
            repo.set_head("refs/heads/main").unwrap();
        }
    }

    // Push to the test node via git smart HTTP.
    let mut remote = repo
        .remote("origin", node_git_url)
        .expect("create remote");

    let mut push_opts = git2::PushOptions::new();
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, _user, allowed| {
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            git2::Cred::userpass_plaintext("did:plc:testuser", "")
        } else {
            Err(git2::Error::from_str("no supported credential type"))
        }
    });
    let push_errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let errors_clone = push_errors.clone();
    callbacks.push_update_reference(move |refname, status| {
        if let Some(err) = status {
            errors_clone
                .lock()
                .unwrap()
                .push(format!("{refname}: {err}"));
        }
        Ok(())
    });
    push_opts.remote_callbacks(callbacks);

    remote
        .push(&["refs/heads/main:refs/heads/main"], Some(&mut push_opts))
        .expect("push to test node");

    let errs = push_errors.lock().unwrap();
    assert!(errs.is_empty(), "destination rejected refs: {errs:?}");

    (commit_id.to_string(), tmp)
}

/// A single commit specification for `seed_git_repo_with_history`.
pub struct CommitSpec<'a> {
    pub message: &'a str,
    /// Files to write for this commit. Entries completely replace the
    /// working-copy contents: anything not listed is deleted.
    pub files: &'a [(&'a str, &'a str)],
}

/// Create a git repository with a linear history of multiple commits,
/// then push it to the given cospan-node URL. Returns the list of git
/// OIDs in the order the commits were made (oldest → newest).
pub async fn seed_git_repo_with_history(
    node_git_url: &str,
    commits: &[CommitSpec<'_>],
) -> (Vec<String>, TempDir) {
    let url = node_git_url.to_string();
    let owned: Vec<(String, Vec<(String, String)>)> = commits
        .iter()
        .map(|c| {
            (
                c.message.to_string(),
                c.files
                    .iter()
                    .map(|(p, cnt)| (p.to_string(), cnt.to_string()))
                    .collect(),
            )
        })
        .collect();
    tokio::task::spawn_blocking(move || seed_history_blocking(&url, &owned))
        .await
        .expect("seed_git_repo_with_history task panicked")
}

fn seed_history_blocking(
    node_git_url: &str,
    commits: &[(String, Vec<(String, String)>)],
) -> (Vec<String>, TempDir) {
    use git2::{IndexAddOption, Repository, Signature};

    let tmp = TempDir::new().unwrap();
    let repo_dir: PathBuf = tmp.path().join("src-repo");
    std::fs::create_dir_all(&repo_dir).unwrap();
    let repo = Repository::init(&repo_dir).unwrap();

    let mut oids: Vec<String> = Vec::with_capacity(commits.len());
    let sig = Signature::now("Test", "test@example.com").unwrap();

    for (i, (message, files)) in commits.iter().enumerate() {
        // Wipe the working copy (except .git) and write new files.
        for entry in std::fs::read_dir(&repo_dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(&path);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        for (path, content) in files {
            let full = repo_dir.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full, content).unwrap();
        }

        let mut index = repo.index().unwrap();
        // Clear and re-add everything.
        index.clear().unwrap();
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let parents: Vec<git2::Commit<'_>> = if i == 0 {
            Vec::new()
        } else {
            let parent = repo
                .find_commit(git2::Oid::from_str(&oids[i - 1]).unwrap())
                .unwrap();
            vec![parent]
        };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

        let commit_id = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap();
        oids.push(commit_id.to_string());

        if i == 0 {
            // Ensure the branch is named main.
            let head = repo.head().unwrap();
            if let Some(name) = head.shorthand() {
                if name != "main" {
                    repo.branch("main", &repo.find_commit(commit_id).unwrap(), false)
                        .unwrap();
                    repo.set_head("refs/heads/main").unwrap();
                }
            }
        }
    }

    // Push the whole history.
    let mut remote = repo
        .remote("origin", node_git_url)
        .expect("create remote");
    let mut push_opts = git2::PushOptions::new();
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, _user, allowed| {
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            git2::Cred::userpass_plaintext("did:plc:testuser", "")
        } else {
            Err(git2::Error::from_str("no supported credential type"))
        }
    });
    push_opts.remote_callbacks(callbacks);
    remote
        .push(&["refs/heads/main:refs/heads/main"], Some(&mut push_opts))
        .expect("push history to test node");

    (oids, tmp)
}

/// Clone a git repository from the given node URL into a fresh temp
/// dir. Safe to call from an async context via `spawn_blocking`.
pub async fn clone_from_node(node_git_url: &str) -> (git2::Repository, TempDir) {
    let url = node_git_url.to_string();
    tokio::task::spawn_blocking(move || clone_from_node_blocking(&url))
        .await
        .expect("clone_from_node task panicked")
}

fn clone_from_node_blocking(node_git_url: &str) -> (git2::Repository, TempDir) {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().join("cloned");
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, _user, allowed| {
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            git2::Cred::userpass_plaintext("did:plc:testuser", "")
        } else {
            Err(git2::Error::from_str("no supported credential type"))
        }
    });
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    let repo = builder
        .clone(node_git_url, &repo_dir)
        .expect("clone from node");
    (repo, tmp)
}
