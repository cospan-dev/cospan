use std::collections::HashMap;
use std::path::PathBuf;

use panproto_core::vcs::{FsStore, HeadState, Object, ObjectId, Store, VcsError};

/// Manages multiple panproto-vcs repositories on disk.
///
/// Each repo lives at `{root}/{did}/{repo_name}/`.
/// Wraps panproto-vcs FsStore directly -- no custom filesystem layer.
pub struct RepoManager {
    root: PathBuf,
}

impl RepoManager {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn repo_dir(&self, did: &str, name: &str) -> PathBuf {
        self.root.join(did).join(name)
    }

    /// Path to the persistent bare git mirror for a repo. This is a
    /// companion to the panproto-vcs FsStore: git smart HTTP endpoints
    /// serve refs and packs from here so we avoid re-exporting panproto
    /// objects on every request (the export is not deterministic and
    /// would produce different git OIDs each time).
    pub fn git_mirror_dir(&self, did: &str, name: &str) -> PathBuf {
        self.repo_dir(did, name).join(".git-mirror")
    }

    /// Open or initialize the bare git mirror for a repo.
    pub fn open_or_init_git_mirror(
        &self,
        did: &str,
        name: &str,
    ) -> Result<git2::Repository, git2::Error> {
        let path = self.git_mirror_dir(did, name);
        if path.exists() {
            git2::Repository::open_bare(&path)
        } else {
            std::fs::create_dir_all(&path).map_err(|e| {
                git2::Error::from_str(&format!("failed to create git mirror dir: {e}"))
            })?;
            git2::Repository::init_bare(&path)
        }
    }

    /// Check if the git mirror exists for a repo.
    pub fn has_git_mirror(&self, did: &str, name: &str) -> bool {
        self.git_mirror_dir(did, name).exists()
    }

    /// Open an existing repo's FsStore.
    pub fn open(&self, did: &str, name: &str) -> Result<FsStore, VcsError> {
        let dir = self.repo_dir(did, name);
        FsStore::open(&dir)
    }

    /// Initialize a new repo, creating the .panproto directory.
    pub fn init(&self, did: &str, name: &str) -> Result<FsStore, VcsError> {
        let dir = self.repo_dir(did, name);
        std::fs::create_dir_all(&dir).map_err(VcsError::Io)?;
        FsStore::init(&dir)
    }

    /// Open or initialize a repo.
    pub fn open_or_init(&self, did: &str, name: &str) -> Result<FsStore, VcsError> {
        match self.open(did, name) {
            Ok(store) => Ok(store),
            Err(VcsError::NotARepository) => self.init(did, name),
            Err(e) => Err(e),
        }
    }

    /// Check if a repo exists.
    pub fn exists(&self, did: &str, name: &str) -> bool {
        self.repo_dir(did, name).join(".panproto").exists()
    }

    /// Get an object from a repo by ID.
    pub fn get_object(&self, did: &str, name: &str, id: &ObjectId) -> Result<Object, VcsError> {
        let store = self.open(did, name)?;
        store.get(id)
    }

    /// Store an object in a repo, returning its content-addressed ID.
    pub fn put_object(&self, did: &str, name: &str, object: &Object) -> Result<ObjectId, VcsError> {
        let mut store = self.open_or_init(did, name)?;
        store.put(object)
    }

    /// Check if a repo has an object.
    pub fn has_object(&self, did: &str, name: &str, id: &ObjectId) -> Result<bool, VcsError> {
        let store = self.open(did, name)?;
        Ok(store.has(id))
    }

    /// Get a ref target.
    pub fn get_ref(
        &self,
        did: &str,
        name: &str,
        ref_name: &str,
    ) -> Result<Option<ObjectId>, VcsError> {
        let store = self.open(did, name)?;
        store.get_ref(ref_name)
    }

    /// Set a ref to point at an object.
    pub fn set_ref(
        &self,
        did: &str,
        name: &str,
        ref_name: &str,
        target: ObjectId,
    ) -> Result<(), VcsError> {
        let mut store = self.open(did, name)?;
        store.set_ref(ref_name, target)
    }

    /// List all refs in a repo.
    ///
    /// NOTE: panproto-vcs `FsStore::list_refs("")` would walk the whole
    /// store root (objects/, HEAD, etc.) and fail on binary files, so
    /// we scope the listing to the `refs/` directory.
    pub fn list_refs(&self, did: &str, name: &str) -> Result<Vec<(String, ObjectId)>, VcsError> {
        let store = self.open(did, name)?;
        store.list_refs("refs/")
    }

    /// Get HEAD state.
    pub fn get_head(&self, did: &str, name: &str) -> Result<HeadState, VcsError> {
        let store = self.open(did, name)?;
        store.get_head()
    }

    /// Path to the git-import marks file. Stores a mapping from git
    /// commit OIDs to panproto-vcs ObjectIds so that subsequent imports
    /// can skip already-imported commits via `import_git_repo_incremental`.
    fn marks_path(&self, did: &str, name: &str) -> PathBuf {
        self.repo_dir(did, name).join(".git-import-marks")
    }

    /// Load the git→panproto OID mapping from the marks file.
    /// Returns an empty map if the file doesn't exist or is corrupt.
    pub fn load_import_marks(
        &self,
        did: &str,
        name: &str,
    ) -> HashMap<git2::Oid, ObjectId> {
        let path = self.marks_path(did, name);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return HashMap::new(),
        };
        let entries: Vec<(String, String)> = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return HashMap::new(),
        };
        let mut map = HashMap::with_capacity(entries.len());
        for (git_hex, pp_hex) in entries {
            let git_oid = match git2::Oid::from_str(&git_hex) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let pp_id = match pp_hex.parse::<ObjectId>() {
                Ok(id) => id,
                Err(_) => continue,
            };
            map.insert(git_oid, pp_id);
        }
        map
    }

    /// Persist the git→panproto OID mapping to the marks file.
    /// Merges new entries with existing ones.
    pub fn save_import_marks(
        &self,
        did: &str,
        name: &str,
        new_entries: &[(git2::Oid, ObjectId)],
    ) {
        let mut map = self.load_import_marks(did, name);
        for (git_oid, pp_id) in new_entries {
            map.insert(*git_oid, *pp_id);
        }
        let entries: Vec<(String, String)> = map
            .into_iter()
            .map(|(g, p)| (g.to_string(), p.to_string()))
            .collect();
        let path = self.marks_path(did, name);
        if let Ok(json) = serde_json::to_vec(&entries) {
            let _ = std::fs::write(&path, json);
        }
    }
}
