use thiserror::Error;

pub use crate::dindex::DIndexVersionId;
use crate::dindex::{self, DIndex};
use sha2::{Digest, Sha256};
use std::{
    fmt::Debug,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

pub struct DIndexManager {
    data_root: PathBuf,
}

#[derive(Debug, Error)]
pub enum DIndexLoadError {
    #[error("I/O Error loading DIndex")]
    Io(#[from] io::Error),
    #[error("Error deserializing DIndex")]
    Deserialization(#[from] dindex::DeserializationError),
    #[error("DIndex does not exist")]
    Nonexistent,
}

impl DIndexManager {
    pub fn new(data_root: impl AsRef<Path>) -> DIndexManager {
        DIndexManager {
            data_root: data_root.as_ref().to_path_buf(),
        }
    }

    pub fn create_data_root(&self) -> io::Result<()> {
        fs::create_dir_all(&self.data_root)
    }

    pub fn data_root(&self) -> &Path {
        self.data_root.as_path()
    }

    fn load_dindex(&self, name: &str) -> Result<DIndex, DIndexLoadError> {
        let path = Path::new(&self.data_root).join(hex::encode(Sha256::digest(name)));
        let file = File::open(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DIndexLoadError::Nonexistent
            } else {
                DIndexLoadError::Io(e)
            }
        })?;

        let mut data = Vec::new();
        zstd::stream::copy_decode(file, &mut data)?;
        DIndex::try_from(data).map_err(Into::into)
    }
    fn persist_dindex(&self, index: DIndex) -> io::Result<()> {
        let name_hash: String = hex::encode(Sha256::digest(index.name()));
        let path = Path::new(&self.data_root).join(name_hash);
        let file = File::create(path)?;
        let data: &[u8] = &Vec::<u8>::from(index);
        zstd::stream::copy_encode(data, file, 3)?;
        Ok(())
    }

    pub fn get_head(&self, name: &str) -> Result<DIndexVersionId, DIndexLoadError> {
        let index = self.load_dindex(name)?;
        Ok(index.head())
    }

    pub fn get_version(
        &self,
        name: &str,
        key: DIndexVersionId,
    ) -> Result<Option<String>, DIndexLoadError> {
        let index = self.load_dindex(name)?;
        Ok(index.get_version_data(key))
    }

    pub fn insert(&self, name: &str, data: &str) -> Result<DIndexVersionId, DIndexLoadError> {
        let result = self.load_dindex(name);

        let index = if let Ok(mut result) = result {
            result.insert_version(data);
            result
        } else {
            DIndex::new(name, data)
        };

        let version_id = index.head();

        self.persist_dindex(index)?;
        Ok(version_id)
    }
}

#[cfg(test)]
mod test {
    use assert_fs::fixture::PathChild;

    use crate::index_manager::DIndexManager;

    const FILE_NAME: &str = "file.txt";

    const FILE_VERSIONS: [&str; 5] = [
        "lines\nof\nthe\nfile\n",
        "the\nfile\n",
        "the\nfile\nlines\nof\n",
        "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n",
        "whole\ndifferent\ntext\n",
    ];

    #[test]
    fn create_one_file() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let dir = tmp.child("indexes");
        let data_root: &str = &dir.path().to_string_lossy();

        let manager = DIndexManager::new(data_root);
        manager.create_data_root().unwrap();

        let version_id = manager.insert(FILE_NAME, FILE_VERSIONS[0]).unwrap();

        let version = manager.get_version(FILE_NAME, version_id).unwrap().unwrap();
        assert_eq!(FILE_VERSIONS[0], version);
    }

    #[test]
    fn modify_one_file() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let dir = tmp.child("indexes");
        let data_root: &str = &dir.path().to_string_lossy();

        let manager = DIndexManager::new(data_root);
        manager.create_data_root().unwrap();

        // insert the root version
        manager.insert(FILE_NAME, FILE_VERSIONS[0]).unwrap();

        let mut version_ids = Vec::with_capacity(FILE_VERSIONS.len());

        // simulate file updates
        for version in FILE_VERSIONS {
            let version_id = manager.insert(FILE_NAME, version).unwrap();
            version_ids.push(version_id);
            let stored_data = manager.get_version(FILE_NAME, version_id).unwrap().unwrap();
            assert_eq!(version, stored_data);
        }

        for i in 0..FILE_VERSIONS.len() {
            let stored_data = manager
                .get_version(FILE_NAME, version_ids[i])
                .unwrap()
                .unwrap();
            assert_eq!(stored_data, FILE_VERSIONS[i])
        }
    }
}
