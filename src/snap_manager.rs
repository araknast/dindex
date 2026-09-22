use hex::FromHexError;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{
    blob_manager::{self, BlobManager},
    dindex::{self, DIndex, DIndexVersionId},
    dpack_manager::{self, DPackLoadError, DPackManager, DPackPersistError},
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
    #[cfg(test)]
    fn get_version_id(&self, path: impl AsRef<Path>) -> Option<&DIndexVersionId> {
        self.entries.get(path.as_ref())
    }
    fn into_entries(self) -> HashMap<PathBuf, DIndexVersionId> {
        self.entries
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
    #[error("Could not insert into DIndex: DIndex does not exist")]
    Nonexistent,
    #[error("Could not insert into DIndex: could not load DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not insert into DIndex: could not persist DPack")]
    DPackPersist(#[from] DPackPersistError),
}

#[derive(Debug, Error)]
pub enum SnapshotLoadError {
    #[error("Error reading snapshot data")]
    Read(#[from] SnapshotReadError),
    #[error("Could not load snapshot: could not load the snapshot index")]
    IndexLoad(#[from] SnapshotIndexLoadError),
    #[error("Could not load snapshot: snapshot does not exist")]
    Nonexistent,
}

#[derive(Debug, Error)]
pub enum SnapshotReproductionError {
    #[error("Could not reproduce snapshot: could not load snapshot")]
    SnapshotLoad(#[from] SnapshotLoadError),
    #[error("Could not reproduce snapshot: could not load a DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not reproduce snapshot: I/O error")]
    Io(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum SnapshotIndexLoadError {
    #[error("Could not load the snapshot DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Snapshot index does not exist in snapshot DPack!")]
    NoIndex,
}

#[derive(Debug, Error)]
#[error("Could not persist snapshot index")]
pub struct SnapshotIndexPersistError(#[from] DPackPersistError);

#[derive(Debug, Error)]
pub enum SnapshotCreationError {
    #[error("I/O error attempting to create snapshot")]
    Io(#[from] io::Error),
    #[error("Could not create snapshot: could not update a file's Dindex")]
    DIndexInsert(#[from] DIndexInsertError),
    #[error("Could not create snapshot: could not persist snapshot")]
    SnapshotPersist(#[from] SnapshotPersistError),
    #[error("Could not create snapshot: could not get head")]
    GetHead(#[from] GetHeadError),
    #[error("Could not create snapshot: invalid target directory")]
    InvalidTarget,
}

#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("Could not initialize dpack manager")]
    DPackManager(#[from] dpack_manager::InitializationError),
    #[error("Could not initialize blob manager")]
    BlobManager(#[from] blob_manager::InitializationError),
}

#[derive(Debug, Error)]
pub enum GetHeadError {
    #[error("Could not get head: could not load snap index")]
    IndexLoad(#[from] SnapshotIndexLoadError),
}

#[derive(Debug, Error)]
pub enum DIndexInsertError {
    #[error("Could not insert into DIndex: DIndex does not exist")]
    Nonexistent,
    #[error("Could not insert into DIndex: could not load DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not insert into DIndex: could not persist DPack")]
    DPackPersist(#[from] DPackPersistError),
}
pub struct SnapshotManager {
    data_dpack_manager: DPackManager,
    snap_dpack_manager: DPackManager,
    blob_manager: BlobManager,
}

impl SnapshotManager {
    const SNAP_INDEX_NAME: &str = "__snap_index";
    pub fn new(data_root: impl AsRef<Path>) -> Result<SnapshotManager, InitializationError> {
        let snap_root = data_root.as_ref().join("snaps");
        let blob_root = data_root.as_ref().join("blobs");
        let data_dpack_manager = DPackManager::new(data_root)?;
        let snap_dpack_manager = DPackManager::new(snap_root)?;
        let blob_manager = BlobManager::new(blob_root)?;
        Ok(SnapshotManager {
            data_dpack_manager,
            snap_dpack_manager,
            blob_manager,
        })
    }

    fn get_head(&self) -> Result<DIndexVersionId, GetHeadError> {
        let snap_index = self.load_snap_index()?;
        Ok(snap_index.head())
    }

    fn insert_into_dindex(
        &mut self,
        name: &str,
        data: &str,
    ) -> Result<DIndexVersionId, DIndexInsertError> {
        let (version_id, dindex) = match self.data_dpack_manager.try_load(&name)? {
            Some(mut index) => (index.insert_version(&data), index),
            None => {
                let index = DIndex::new(name, data);
                (index.head(), index)
            }
        };

        self.data_dpack_manager.try_persist(dindex)?;
        Ok(version_id)
    }

    fn load_snap_index(&self) -> Result<DIndex, SnapshotIndexLoadError> {
        self.snap_dpack_manager
            .try_load(Self::SNAP_INDEX_NAME)?
            .ok_or(SnapshotIndexLoadError::NoIndex)
    }

    fn get_snapshot_by_id(&self, id: DIndexVersionId) -> Result<Snapshot, SnapshotLoadError> {
        let snap_index = self.load_snap_index()?;
        let snap_data = snap_index.get_version_data(id);
        if let Some(snap_data) = snap_data {
            Ok(Snapshot::try_from(snap_data)?)
        } else {
            Err(SnapshotLoadError::Nonexistent)
        }
    }

    fn persist_snapshot(
        &mut self,
        snap: Snapshot,
    ) -> Result<DIndexVersionId, SnapshotPersistError> {
        let data: String = snap.into();
        let (version_id, dindex) = match self.snap_dpack_manager.try_load(Self::SNAP_INDEX_NAME)? {
            Some(mut index) => (index.insert_version(&data), index),
            None => {
                let index = DIndex::new(Self::SNAP_INDEX_NAME, &data);
                (index.head(), index)
            }
        };

        self.snap_dpack_manager.try_persist(dindex)?;
        Ok(version_id)
    }

    fn process_dir(
        &mut self,
        dir: &Path,
        basename: impl AsRef<Path>,
        snap: &mut Snapshot,
        ignored_paths: &Vec<impl AsRef<Path>>,
    ) -> Result<(), SnapshotCreationError> {
        'a: for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            for ignored_path in ignored_paths {
                if ignored_path.as_ref().file_name() == path.file_name() {
                    continue 'a;
                }
            }
            let relative_path = basename.as_ref().join(
                path.file_name()
                    .expect("Path read from dir had an invalid filename!"),
            );
            if path.is_dir() {
                self.process_dir(&path, relative_path, snap, ignored_paths)?;
            } else if !&snap.contains_path(&path) {
                let path_string = &relative_path.as_os_str().to_string_lossy();
                let version = match fs::read_to_string(&path) {
                    Ok(data) => self.insert_into_dindex(path_string, &data)?,
                    Err(e) if e.kind() == io::ErrorKind::InvalidData => self
                        .blob_manager
                        .insert_blob(path_string, fs::read(&path)?)?,
                    Err(e) => return Err(e.into()),
                };
                snap.update_entry(relative_path, version);
            }
        }
        Ok(())
    }
    // Creates a snapshot from the contents of a directory, probably not the
    // final interface
    pub fn snapshot_from_dir(
        &mut self,
        path: impl AsRef<Path>,
        ignored_paths: Vec<impl AsRef<Path>>,
    ) -> Result<DIndexVersionId, SnapshotCreationError> {
        let parent_id = match self.get_head() {
            Ok(id) => Some(id),
            Err(GetHeadError::IndexLoad(SnapshotIndexLoadError::NoIndex)) => None,
            Err(e) => return Err(e.into()),
        };
        let mut snap = Snapshot::new(parent_id);

        self.process_dir(path.as_ref(), "", &mut snap, &ignored_paths)?;
        self.persist_snapshot(snap).map_err(Into::into)
    }

    pub fn snapshot_into_dir(
        &self,
        id: DIndexVersionId,
        target: impl AsRef<Path>,
    ) -> Result<(), SnapshotReproductionError> {
        let snap = self.get_snapshot_by_id(id)?;
        for (path, version_id) in snap.into_entries() {
            let full_path = target.as_ref().join(&path);
            let dindex_name = path.to_string_lossy();

            let data: Vec<u8> = match self.data_dpack_manager.try_load(&dindex_name) {
                Ok(Some(dindex)) => dindex
                    .get_version_data(version_id)
                    .expect("Version in snapshot does not exist in DIndex!")
                    .into(),
                Ok(None) => self
                    .blob_manager
                    .get_blob_version(&dindex_name, version_id)?,
                Err(e) => return Err(e.into()),
            };

            match fs::write(&full_path, data) {
                Ok(()) => continue,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir_all(&full_path.parent().expect("Invalid path in snapshot!"))?;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
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

    const BLOB_FILE: [u8; 1] = [255];

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

        let snapshot_manager = SnapshotManager::new(index_dir).unwrap();

        (tmp, data_dir, snapshot_manager)
    }

    fn initialize_test_dir_recursive() -> (TempDir, ChildPath, ChildPath, SnapshotManager) {
        let tmp = assert_fs::TempDir::new().unwrap();
        let index_dir = tmp.child("indexes");
        let data_dir = tmp.child("data");
        let subdir = data_dir.child("dir");

        fs::create_dir_all(&index_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(&subdir).unwrap();

        for name in FILE_NAMES {
            let file_path = data_dir.path().to_path_buf().join(name);
            fs::write(file_path, FILE_VERSIONS[0]).unwrap();
        }

        for name in FILE_NAMES {
            let file_path = subdir.path().to_path_buf().join(name);
            fs::write(file_path, FILE_VERSIONS[0]).unwrap();
        }

        let snapshot_manager = SnapshotManager::new(index_dir).unwrap();

        (tmp, data_dir, subdir, snapshot_manager)
    }

    fn new_snap_object(mut manager: SnapshotManager, data_dir: &Path) -> Snapshot {
        let snap_id = manager
            .snapshot_from_dir(data_dir, Vec::<String>::new())
            .unwrap();
        manager.get_snapshot_by_id(snap_id).unwrap()
    }

    #[test]
    fn test_into_dir_recursive() {
        let (tmp, data_dir, _, mut manager) = initialize_test_dir_recursive();
        let output_dir = tmp.child("output");
        fs::create_dir_all(&output_dir).unwrap();
        let snap_id = manager
            .snapshot_from_dir(&data_dir, Vec::<String>::new())
            .unwrap();
        manager.snapshot_into_dir(snap_id, &output_dir).unwrap();
        let output_dirents: Vec<_> = fs::read_dir(output_dir).unwrap().collect();
        let base_dirents: Vec<_> = fs::read_dir(data_dir).unwrap().collect();

        assert!(output_dirents.len() == base_dirents.len());
        for i in 0..output_dirents.len() {
            assert!(output_dirents[i].is_ok() && base_dirents[i].is_ok())
        }
    }

    #[test]
    fn test_into_dir() {
        let (tmp, data_dir, mut manager) = initialize_test_dir();
        let output_dir = tmp.child("output");
        fs::create_dir_all(&output_dir).unwrap();
        let snap_id = manager
            .snapshot_from_dir(&data_dir, Vec::<String>::new())
            .unwrap();
        manager.snapshot_into_dir(snap_id, &output_dir).unwrap();
        let output_dirents: Vec<_> = fs::read_dir(output_dir).unwrap().collect();
        let base_dirents: Vec<_> = fs::read_dir(data_dir).unwrap().collect();
        assert!(output_dirents.len() == base_dirents.len());
        for i in 0..output_dirents.len() {
            assert!(output_dirents[i].is_ok() && base_dirents[i].is_ok())
        }
    }

    #[test]
    fn test_into_dir_blob() {
        let (tmp, data_dir, mut manager) = initialize_test_dir();

        let blob_path = "blob.bin";
        fs::write(&data_dir.join(&blob_path), BLOB_FILE).unwrap();

        let output_dir = tmp.child("output");
        fs::create_dir_all(&output_dir).unwrap();

        let snap_id = manager
            .snapshot_from_dir(&data_dir, Vec::<String>::new())
            .unwrap();
        manager.snapshot_into_dir(snap_id, &output_dir).unwrap();

        let output_dirents: Vec<_> = fs::read_dir(output_dir).unwrap().collect();
        let base_dirents: Vec<_> = fs::read_dir(data_dir).unwrap().collect();
        assert!(output_dirents.len() == base_dirents.len());
        for i in 0..output_dirents.len() {
            assert!(output_dirents[i].is_ok() && base_dirents[i].is_ok())
        }
    }

    #[test]
    fn test_to_from_string() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = new_snap_object(manager, data_dir.path());
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

        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            fs::write(full_path, FILE_VERSIONS[i + 1]).unwrap();
        }

        let snap = new_snap_object(manager, data_dir.path());
        let snap_string = String::from(snap.clone());
        let snap_from_string = Snapshot::try_from(snap_string).unwrap();

        for (path, _) in &snap.entries {
            assert!(snap_from_string.contains_path(path));
            assert!(snap_from_string.entries.get(path) == snap.entries.get(path));
        }
    }

    #[test]
    fn test_new_snapshot_with_blob() {
        let (_tmp, data_dir, manager) = initialize_test_dir();

        let blob_path = "blob.bin";
        fs::write(&data_dir.join(&blob_path), BLOB_FILE).unwrap();

        let snap = new_snap_object(manager, data_dir.path());
        let v1_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        for path in FILE_NAMES {
            assert!(snap.contains_path(&path));
            assert!(*snap.get_version_id(&path).unwrap() == v1_id);
        }
        let blob_id = DIndexVersionId::from_version_data(BLOB_FILE);
        assert!(snap.contains_path(&blob_path));
        assert!(*snap.get_version_id(&blob_path).unwrap() == blob_id);
    }

    #[test]
    fn test_new_snapshot() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        let snap = new_snap_object(manager, data_dir.path());
        let v1_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        for path in FILE_NAMES {
            assert!(snap.contains_path(&path));
            assert!(*snap.get_version_id(&path).unwrap() == v1_id);
        }
    }
    #[test]
    fn test_update_same_files() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            fs::write(full_path, FILE_VERSIONS[i + 1]).unwrap();
        }
        let snap = new_snap_object(manager, &data_dir.path());
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[i + 1]);
            assert!(snap.contains_path(&path));
            assert!(*snap.get_version_id(&path).unwrap() == expected_id);
        }
    }
    #[test]
    fn test_update_new_files() {
        let (_tmp, data_dir, manager) = initialize_test_dir();
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let full_path = data_dir.path().to_path_buf().join(path);
            fs::write(full_path, FILE_VERSIONS[i + 1]).unwrap();
        }
        let new_file_path = "new_file.txt";
        let full_new_file_path = data_dir.path().to_path_buf().join(new_file_path);
        let new_file_expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        fs::write(&full_new_file_path, FILE_VERSIONS[0]).unwrap();

        let snap = new_snap_object(manager, &data_dir.path());
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[i + 1]);
            assert!(snap.contains_path(&path));
            assert!(*snap.get_version_id(&path).unwrap() == expected_id);
        }
        assert!(snap.contains_path(&new_file_path));
        assert!(*snap.get_version_id(&new_file_path).unwrap() == new_file_expected_id);
    }
    #[test]
    fn test_update_removed_files() {
        let (_tmp, data_dir, manager) = initialize_test_dir();

        let removed_path = FILE_NAMES[2];
        let full_removed_path = data_dir.path().to_path_buf().join(removed_path);
        fs::remove_file(&full_removed_path).unwrap();

        let snap = new_snap_object(manager, &data_dir.path());
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            if path != removed_path {
                let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
                assert!(snap.contains_path(&path));
                assert!(*snap.get_version_id(&path).unwrap() == expected_id);
            } else {
                assert!(!snap.contains_path(&path))
            }
        }
    }
    #[test]
    fn test_update_file_now_directory() {
        let (_tmp, data_dir, manager) = initialize_test_dir();

        let directory_path = FILE_NAMES[2];
        let full_directory_path = data_dir.path().to_path_buf().join(directory_path);
        fs::remove_file(&full_directory_path).unwrap();
        fs::create_dir(&full_directory_path).unwrap();

        let snap = new_snap_object(manager, &data_dir.path());
        for i in 0..FILE_NAMES.len() {
            let path = FILE_NAMES[i];
            if path != directory_path {
                let expected_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
                assert!(snap.contains_path(&path));
                assert!(*snap.get_version_id(&path).unwrap() == expected_id);
            } else {
                assert!(!snap.contains_path(&path))
            }
        }
    }
}
