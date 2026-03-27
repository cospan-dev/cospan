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
    pub fn list_refs(&self, did: &str, name: &str) -> Result<Vec<(String, ObjectId)>, VcsError> {
        let store = self.open(did, name)?;
        store.list_refs("")
    }

    /// Get HEAD state.
    pub fn get_head(&self, did: &str, name: &str) -> Result<HeadState, VcsError> {
        let store = self.open(did, name)?;
        store.get_head()
    }
}
