use thiserror::Error;

use crate::manager::dindex::{DIndex, DIndexObjectId, DeserializationError};
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

    fn get(&self, name: &str, key: DIndexObjectId) -> Result<Option<String>, DIndexLoadError> {
        let index = self.load_dindex(name)?;
        Ok(index.get_object_data(key))
    }

    fn insert(
        &self,
        name: &str,
        data: &str,
        parent: DIndexObjectId,
    ) -> Result<DIndexObjectId, DIndexLoadError> {
        let result = self.load_dindex(name);

        let mut index = if let Err(DIndexLoadError::Io(_)) = result {
            DIndex::new(name)
        } else {
            result?
        };

        let object_id = index.insert_object(data, parent);
        self.persist_dindex(index)?;
        Ok(object_id)
    }
}

#[cfg(test)]
mod test {
    use assert_fs::fixture::PathChild;

    use crate::manager::{DIndexManager, dindex::DIndexObjectId};

    const FILE1: &str = "lines\nof\nthe\nfile\n";
    const FILE2: &str = "the\nfile\n";
    const FILE3: &str = "the\nfile\nlines\nof\n";
    const FILE4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const FILE5: &str = "whole\ndifferent\ntext\n";

    #[test]
    fn create_new_dindex() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let dir = tmp.child("indexes");
        let data_root: &str = &dir.path().to_string_lossy();

        let manager = DIndexManager::new(data_root);
        manager.create_data_root().unwrap();

        let parent_id = DIndexObjectId::from_object_data(FILE1);
        let object_id = manager.insert("file.txt", FILE1, parent_id).unwrap();

        let object = manager.get("file.txt", object_id).unwrap().unwrap();
        assert_eq!(FILE1, object);
    }
}
