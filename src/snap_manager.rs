use hex::FromHexError;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{
    dindex::{self, DIndexVersionId},
    index_manager::{DIndexLoadError, DIndexManager},
};

#[derive(Debug, Clone)]
struct Snapshot {
    entries: HashMap<PathBuf, DIndexVersionId>,
    parent_id: Option<DIndexVersionId>,
}
impl Snapshot {
    fn new(parent_id: Option<DIndexVersionId>) -> Snapshot {
        Snapshot {
            entries: HashMap::new(),
            parent_id,
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
        if let Some(parent_id) = snap.parent_id {
            string.push_str(&hex::encode(parent_id));
            string.push_str("\n");
        } else {
            string.push('\0');
            string.push_str("\n");
        }
        for (path, id_str) in snap.entries {
            let name = path.as_os_str().to_string_lossy();
            string.push_str(&name);
            string.push_str(" ");
            string.push_str(&hex::encode(id_str));
            string.push_str("\n");
        }
        string
    }
}

impl TryFrom<String> for Snapshot {
    type Error = SnapshotReadError;
    fn try_from(data: String) -> Result<Snapshot, Self::Error> {
        let mut entries = HashMap::new();
        let mut iter = data.lines();
        let parent_id_str = iter.next().ok_or(SnapshotReadError::EarlyTermination)?;
        let parent_id: Option<DIndexVersionId> = if parent_id_str == "\0" {
            None
        } else {
            Some(hex::decode(parent_id_str)?.try_into()?)
        };

        for line in iter {
            let mut split = line.split(" ");
            let name = split.next().ok_or(SnapshotReadError::EarlyTermination)?;
            let id: DIndexVersionId =
                hex::decode(split.next().ok_or(SnapshotReadError::EarlyTermination)?)?
                    .try_into()?;
            entries.insert(PathBuf::from(name), id);
        }

        Ok(Snapshot { parent_id, entries })
    }
}

#[derive(Debug, Error)]
#[error("Failed to parse snapshot data")]
pub enum SnapshotReadError {
    #[error("Could not parse snapshot id")]
    ObjectIdParse(#[from] FromHexError),
    #[error("File ended early")]
    EarlyTermination,
    #[error("Invalid snapshot id")]
    InvalidId(#[from] dindex::DeserializationError),
}

#[derive(Debug, Error)]
pub enum SnapshotPersistError {
    #[error("Could not load the snapshot DIndex")]
    DIndexLoad(#[from] DIndexLoadError),
}

#[derive(Debug, Error)]
pub enum SnapshotLoadError {
    #[error("Error reading snapshot data")]
    Read(#[from] SnapshotReadError),
    #[error("Could not load the snapshot DIndex")]
    DIndexLoad(#[from] DIndexLoadError),
}

#[derive(Debug, Error)]
pub enum SnapshotCreationError {
    #[error("I/O error attempting to create snapshot")]
    Io(#[from] io::Error),
    #[error("Could not load the file's DIndex")]
    DIndexLoad(#[from] DIndexLoadError),
    #[error("Could not load the parent snapshot")]
    ParentSnapLoad(#[from] SnapshotLoadError),
    #[error("Parent snapshot does not exist")]
    ParentSnapDoesNotExist,
    #[error("Could not persist snapshot")]
    SnapshotPersist(#[from] SnapshotPersistError),
}

pub struct SnapshotManager {
    data_index_manager: DIndexManager,
    snap_index_manager: DIndexManager,
}

impl SnapshotManager {
    const SNAP_INDEX_NAME: &str = "__snap_index";
    pub fn new(data_root: impl AsRef<Path>) -> SnapshotManager {
        let manager = DIndexManager::new(data_root);
        let snap_root = Path::new(&manager.data_root()).join("snaps");
        SnapshotManager {
            data_index_manager: manager,
            snap_index_manager: DIndexManager::new(
                &snap_root
                    .into_os_string()
                    .into_string()
                    .expect("snap root path contains invalid unicode"),
            ),
        }
    }

    pub fn init_dirs(&self) -> io::Result<()> {
        self.data_index_manager.create_data_root()?;
        self.snap_index_manager.create_data_root()?;
        Ok(())
    }

    fn get_head(&self) -> Result<DIndexVersionId, DIndexLoadError> {
        self.snap_index_manager.get_head(Self::SNAP_INDEX_NAME)
    }

    fn get_snapshot_by_id(
        &self,
        id: DIndexVersionId,
    ) -> Result<Option<Snapshot>, SnapshotLoadError> {
        let snap_data = self
            .snap_index_manager
            .get_version(Self::SNAP_INDEX_NAME, id)?;
        if let Some(snap_data) = snap_data {
            Ok(Some(Snapshot::try_from(snap_data)?))
        } else {
            Ok(None)
        }
    }

    fn persist_snapshot(&self, snap: Snapshot) -> Result<DIndexVersionId, SnapshotPersistError> {
        let snap_data: String = snap.into();
        self.snap_index_manager
            .insert(Self::SNAP_INDEX_NAME, &snap_data)
            .map_err(Into::into)
    }

    fn update_snapshot(
        &self,
        mut snap: Snapshot,
    ) -> Result<DIndexVersionId, SnapshotCreationError> {
        let mut for_removal = Vec::new();
        for (path, version_id) in &mut snap.entries {
            if !path.is_file() {
                for_removal.push(path.clone());
                continue;
            }
            let new_version_id = self.data_index_manager.insert(
                &path.as_os_str().to_string_lossy(),
                &fs::read_to_string(&path)?,
            )?;

            if new_version_id != *version_id {
                *version_id = new_version_id;
            }
        }

        for path in for_removal {
            snap.entries.remove(&path);
        }
        self.persist_snapshot(snap).map_err(Into::into)
    }
    // Creates a snapshot from the contents of a directory
    pub fn snapshot_from_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<DIndexVersionId, SnapshotCreationError> {
        let parent_id = match self.get_head() {
            Ok(id) => Some(id),
            Err(DIndexLoadError::Nonexistent) => None,
            Err(e) => return Err(e.into()),
        };
        let mut snap = Snapshot::new(parent_id);

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
                    )?;
                    snap.update_entry(&path, version);
                }
            }
            Ok(())
        }
        process_dir(path, &mut snap, &self.data_index_manager)?;
        self.persist_snapshot(snap).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    // Note: indexes and data are currently being kept separate, there is
    // currently no logic for the snapshot manager to ignore an index directory
    use std::{fs, path::Path};

    use assert_fs::{
        TempDir,
        fixture::{ChildPath, PathChild},
    };

    use crate::{
        dindex::DIndexVersionId,
        snap_manager::{Snapshot, SnapshotManager},
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

        let snapshot_manager = SnapshotManager::new(index_dir);
        snapshot_manager.init_dirs().unwrap();

        (tmp, data_dir, snapshot_manager)
    }

    fn new_snap_object(
        manager: SnapshotManager,
        data_dir: &Path,
        parent_id: Option<DIndexVersionId>,
    ) -> Snapshot {
        let snap_id = manager.snapshot_from_dir(data_dir).unwrap();
        manager.get_snapshot_by_id(snap_id).unwrap().unwrap()
    }

    #[test]
    fn test_to_from_string() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = new_snap_object(manager, data_dir.path(), None);
        let snap_string = String::from(snap.clone());
        let snap_from_string = Snapshot::try_from(snap_string).unwrap();

        for (path, _) in &snap.entries {
            assert!(snap_from_string.contains_path(path));
            assert!(snap_from_string.entries.get(path) == snap.entries.get(path));
        }
    }
    #[test]
    fn test_to_from_string_with_parent() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let parent_id = manager.snapshot_from_dir(data_dir.path()).unwrap();

        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            fs::write(full_path, FILE_VERSIONS[i + 1]).unwrap();
        }

        let snap = new_snap_object(manager, data_dir.path(), Some(parent_id));
        let snap_string = String::from(snap.clone());
        let snap_from_string = Snapshot::try_from(snap_string).unwrap();

        for (path, _) in &snap.entries {
            assert!(snap_from_string.contains_path(path));
            assert!(snap_from_string.entries.get(path) == snap.entries.get(path));
        }
    }
    #[test]
    fn test_new_snapshot() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = new_snap_object(manager, data_dir.path(), None);
        let v1_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        for path in FILE_NAMES {
            let full_path = data_dir.path().to_path_buf().join(path);
            assert!(snap.contains_path(&full_path));
            assert!(*snap.get_version_id(&full_path).unwrap() == v1_id);
        }
    }
    #[test]
    fn test_update_same_files() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = manager.snapshot_from_dir(data_dir.path()).unwrap();
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            fs::write(full_path, FILE_VERSIONS[i + 1]).unwrap();
        }
        let snap = new_snap_object(manager, &data_dir.path(), Some(snap));
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[i + 1]);
            assert!(snap.contains_path(&full_path));
            assert!(*snap.get_version_id(&full_path).unwrap() == expected_id);
        }
    }
    #[test]
    fn test_update_new_files() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = manager.snapshot_from_dir(data_dir.path()).unwrap();
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            fs::write(full_path, FILE_VERSIONS[i + 1]).unwrap();
        }
        let new_file_path = data_dir.path().to_path_buf().join("new_file.txt");
        let new_file_expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        fs::write(&new_file_path, FILE_VERSIONS[0]).unwrap();

        let snap = new_snap_object(manager, &data_dir.path(), Some(snap));
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[i + 1]);
            assert!(snap.contains_path(&full_path));
            assert!(*snap.get_version_id(&full_path).unwrap() == expected_id);
        }
        assert!(snap.contains_path(&new_file_path));
        assert!(*snap.get_version_id(&new_file_path).unwrap() == new_file_expected_id);
    }
    #[test]
    fn test_update_removed_files() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = manager.snapshot_from_dir(data_dir.path()).unwrap();

        let removed_path = data_dir.path().to_path_buf().join(FILE_NAMES[2]);
        fs::remove_file(&removed_path).unwrap();

        let snap = new_snap_object(manager, &data_dir.path(), Some(snap));
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            if full_path != removed_path {
                let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
                assert!(snap.contains_path(&full_path));
                assert!(*snap.get_version_id(&full_path).unwrap() == expected_id);
            } else {
                assert!(!snap.contains_path(full_path))
            }
        }
    }
    #[test]
    fn test_update_file_now_directory() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = manager.snapshot_from_dir(data_dir.path()).unwrap();

        let directory_path = data_dir.path().to_path_buf().join(FILE_NAMES[2]);
        fs::remove_file(&directory_path).unwrap();
        fs::create_dir(&directory_path).unwrap();

        let snap = new_snap_object(manager, &data_dir.path(), Some(snap));
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            if full_path != directory_path {
                let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
                assert!(snap.contains_path(&full_path));
                assert!(*snap.get_version_id(&full_path).unwrap() == expected_id);
            } else {
                assert!(!snap.contains_path(full_path))
            }
        }
    }
}
