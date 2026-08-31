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

    fn load_dindex(&self, name: &str) -> Result<DIndex, DIndexLoadError> {
        let path = Path::new(&self.data_root).join(hex::encode(name));
        DIndex::try_from(fs::read(path)?).map_err(Into::into)
    }
    fn persist_dindex(&self, index: DIndex) -> Result<(), io::Error> {
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

        Ok(index.insert_object(data, parent))
    }
}
