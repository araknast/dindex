use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{
    dindex::DIndexVersionId,
    index_manager::{self, DIndexLoadError, DIndexManager},
};

#[derive(Debug)]
struct Snapshot {
    entries: HashMap<PathBuf, DIndexVersionId>,
}
impl Snapshot {
    fn new() -> Snapshot {
        Snapshot {
            entries: HashMap::new(),
        }
    }
    fn update_entry(&mut self, path: impl AsRef<Path>, id: DIndexVersionId) {
        self.entries.insert(path.as_ref().to_path_buf(), id);
    }
    fn contains_path(&self, path: impl AsRef<Path>) -> bool {
        self.entries.contains_key(path.as_ref())
    }
    fn get_version_id(&self, path: impl AsRef<Path>) -> Option<&DIndexVersionId> {
        self.entries.get(path.as_ref())
    }
}
impl From<Snapshot> for String {
    fn from(snap: Snapshot) -> String {
        let mut string = String::new();
        for (path, id_str) in snap.entries {
            let name = path.as_os_str().to_string_lossy();
            let id_str: [u8; 32] = id_str.into();
            string.push_str(&name);
            string.push_str(" ");
            string.push_str(&hex::encode(id_str));
            string.push_str("\n");
        }
        string
    }
}

#[derive(Debug, Error)]
pub enum SnapshotCreationError {
    #[error("I/O Attempting to create snapshot")]
    Io(#[from] io::Error),
    #[error("Could not create snapshot, could not load a file's DIndex")]
    DIndexLoad(#[from] DIndexLoadError),
}
struct SnapshotManager {
    snap_index_path: String,
    head_snap_path: String,
    data_index_manager: DIndexManager,
    snap_index_manager: DIndexManager,
}

impl SnapshotManager {
    const SNAP_INDEX_NAME: &str = "__snap_index";
    pub fn new(manager: DIndexManager) -> SnapshotManager {
        let snap_root = Path::new(&manager.data_root()).join("snaps");
        let head_snap_path = snap_root
            .join("HEAD")
            .into_os_string()
            .into_string()
            .expect("snap head path contains invalid unicode");
        let snap_index_path = snap_root
            .join("index")
            .into_os_string()
            .into_string()
            .expect("snap index path contains invalid unicode");
        SnapshotManager {
            snap_index_path,
            head_snap_path,
            data_index_manager: manager,
            snap_index_manager: DIndexManager::new(
                &snap_root
                    .into_os_string()
                    .into_string()
                    .expect("snap root path contains invalid unicode"),
            ),
        }
    }

    fn get_head(&self) -> Option<DIndexVersionId> {
        None
    }
    fn update_snapshot(&self, mut snap: Snapshot) -> Result<Snapshot, SnapshotCreationError> {
        let mut for_removal = Vec::new();
        for (path, version_id) in &mut snap.entries {
            if !path.is_file() {
                for_removal.push(path.clone());
                continue;
            }
            let new_version_id = self.data_index_manager.insert(
                &path.as_os_str().to_string_lossy(),
                &fs::read_to_string(&path)?,
                None,
            )?;

            if new_version_id != *version_id {
                *version_id = new_version_id;
            }
        }

        for path in for_removal {
            snap.entries.remove(&path);
        }
        Ok(snap)
    }
    // Creates a snapshot from the contents of a directory
    fn snapshot_from_dir(
        &self,
        path: impl AsRef<Path>,
        parent: Option<Snapshot>,
    ) -> Result<Snapshot, SnapshotCreationError> {
        let mut snap = if let Some(parent) = parent {
            self.update_snapshot(parent)?
        } else {
            Snapshot::new()
        };

        fn process_dir(
            dir: impl AsRef<Path>,
            snap: &mut Snapshot,
            index_manager: &DIndexManager,
        ) -> Result<(), SnapshotCreationError> {
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    process_dir(&path, snap, index_manager)?;
                } else if !&snap.contains_path(&path) {
                    let version = index_manager.insert(
                        &path.as_os_str().to_string_lossy(),
                        &fs::read_to_string(&path)?,
                        None,
                    )?;
                    snap.update_entry(&path, version);
                }
            }
            Ok(())
        }
        process_dir(path, &mut snap, &self.data_index_manager)?;
        Ok(snap)
    }
}

#[cfg(test)]
mod test {
    // Note: indexes and data are currently being kept separate, there is
    // currently no logic for the snapshot manager to ignore an index directory
    use std::{fs, path::PathBuf};

    use assert_fs::{
        TempDir,
        fixture::{ChildPath, PathChild},
    };

    use crate::{
        dindex::DIndexVersionId, index_manager::DIndexManager, snap_manager::SnapshotManager,
    };
    const FILE_NAMES: [&str; 3] = ["file1.txt", "file2.txt", "file3.txt"];

    const FILE_VERSIONS: [&str; 5] = [
        "lines\nof\nthe\nfile\n",
        "the\nfile\n",
        "the\nfile\nlines\nof\n",
        "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n",
        "whole\ndifferent\ntext\n",
    ];

    fn initialize_test_dir() -> (TempDir, ChildPath, SnapshotManager) {
        let tmp = assert_fs::TempDir::new().unwrap();
        let index_dir = tmp.child("indexes");
        let data_dir = tmp.child("data");

        fs::create_dir_all(&index_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();
        for name in FILE_NAMES {
            let file_path = data_dir.path().to_path_buf().join(name);
            fs::write(file_path, FILE_VERSIONS[0]).unwrap();
        }

        let index_manager = DIndexManager::new(index_dir);
        let snapshot_manager = SnapshotManager::new(index_manager);

        (tmp, data_dir, snapshot_manager)
    }
    #[test]
    fn test_new_snapshot() {
        let (tmp, data_dir, manager) = initialize_test_dir();
        let snap = manager.snapshot_from_dir(data_dir.path(), None).unwrap();
        let v1_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        for path in FILE_NAMES {
            let full_path = data_dir.path().to_path_buf().join(path);
            assert!(snap.contains_path(&full_path));
            assert!(*snap.get_version_id(&full_path).unwrap() == v1_id);
        }
    }
    fn test_update_same_files() {}
    fn test_update_new_files() {}
    fn test_update_removed_files() {}
    fn test_update_file_now_directory() {}
}
