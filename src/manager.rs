use thiserror::Error;

use crate::manager::dindex::{DIndex, DIndexKey, DeserializationError};
use std::{fs, io, path::Path};

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
        let path = Path::new(&self.data_root).join(name);
        DIndex::try_from(fs::read(path)?).map_err(Into::into)
    }
    fn persist_dindex(&self, index: DIndex) -> Result<(), io::Error> {
        let path = Path::new(&self.data_root).join(index.name());
        fs::write(path, Vec::<u8>::from(index))
    }
    fn get(name: &str, key: DIndexKey) {}
    fn insert(name: &str, data: String) {}
}
