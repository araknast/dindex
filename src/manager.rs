use thiserror::Error;

use crate::manager::dindex::{DIndex, DIndexVersionId, DeserializationError};
use sha2::{Digest, Sha256};
use std::{fmt::Debug, fs, io, path::Path};

mod dindex;
pub struct DIndexManager {
    data_root: String,
}

#[derive(Debug, Error)]
enum DIndexLoadError {
    #[error("I/O Error loading DIndex")]
    Io(#[from] io::Error),
    #[error("Error deserializing DIndex")]
    Deserialization(#[from] DeserializationError),
}

impl DIndexManager {
    pub fn new(data_root: &str) -> DIndexManager {
        DIndexManager {
            data_root: String::from(data_root),
        }
    }

    pub fn create_data_root(&self) -> io::Result<()> {
        fs::create_dir_all(&self.data_root)
    }

    fn load_dindex(&self, name: &str) -> Result<DIndex, DIndexLoadError> {
        let path = Path::new(&self.data_root).join(hex::encode(Sha256::digest(name)));
        DIndex::try_from(fs::read(path)?).map_err(Into::into)
    }
    fn persist_dindex(&self, index: DIndex) -> io::Result<()> {
        let name_hash: String = hex::encode(Sha256::digest(index.name()));
        let path = Path::new(&self.data_root).join(name_hash);
        fs::write(path, Vec::<u8>::from(index))
    }

    fn get(&self, name: &str, key: DIndexVersionId) -> Result<Option<String>, DIndexLoadError> {
        let index = self.load_dindex(name)?;
        Ok(index.get_version_data(key))
    }

    fn insert(
        &self,
        name: &str,
        data: &str,
        parent: DIndexVersionId,
    ) -> Result<DIndexVersionId, DIndexLoadError> {
        let result = self.load_dindex(name);

        let mut index = if let Err(DIndexLoadError::Io(_)) = result {
            DIndex::new(name)
        } else {
            result?
        };

        let version_id = index.insert_version(data, parent);
        self.persist_dindex(index)?;
        Ok(version_id)
    }
}

#[cfg(test)]
mod test {
    use assert_fs::fixture::PathChild;

    use crate::manager::{DIndexManager, dindex::DIndexVersionId};

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

        let parent_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        let version_id = manager
            .insert(FILE_NAME, FILE_VERSIONS[0], parent_id)
            .unwrap();

        let version = manager.get(FILE_NAME, version_id).unwrap().unwrap();
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
        let mut parent_id = DIndexVersionId::from_version_data(FILE_VERSIONS[0]);
        manager
            .insert(FILE_NAME, FILE_VERSIONS[0], parent_id)
            .unwrap();

        let mut version_ids = Vec::with_capacity(FILE_VERSIONS.len());

        // simulate file updates
        for version in FILE_VERSIONS {
            let version_id = manager.insert(FILE_NAME, version, parent_id).unwrap();
            version_ids.push(version_id);
            let stored_data = manager.get(FILE_NAME, version_id).unwrap().unwrap();
            assert_eq!(version, stored_data);
            parent_id = version_id;
        }

        for i in 0..FILE_VERSIONS.len() {
            let stored_data = manager.get(FILE_NAME, version_ids[i]).unwrap().unwrap();
            assert_eq!(stored_data, FILE_VERSIONS[i])
        }
    }
}
