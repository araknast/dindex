use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::dindex::DIndexVersionId;

use std::{
    fmt::Debug,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

pub struct BlobManager {
    data_root: PathBuf,
}

#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("I/O Error initializing blob manager")]
    Io(#[from] io::Error),
}

impl BlobManager {
    pub fn new(data_root: impl AsRef<Path>) -> Result<BlobManager, InitializationError> {
        fs::create_dir_all(data_root.as_ref())?;
        Ok(BlobManager {
            data_root: data_root.as_ref().to_path_buf(),
        })
    }

    fn encode_name(name: &str) -> String {
        hex::encode(Sha256::digest(name))
    }

    pub fn get_blob_version(
        &self,
        name: &str,
        version_id: DIndexVersionId,
    ) -> Result<Vec<u8>, io::Error> {
        let file_name: String = Self::encode_name(name);
        let version_id_string = hex::encode(version_id);
        let dirname = Path::new(&self.data_root).join("bin").join(&file_name);
        let path = dirname.join(&version_id_string);

        let file = File::open(&path)?;

        let mut data = Vec::new();
        zstd::stream::copy_decode(file, &mut data)?;
        Ok(data)
    }

    pub fn insert_blob(&self, name: &str, data: Vec<u8>) -> io::Result<DIndexVersionId> {
        let data: &[u8] = &data;
        let file_name: String = Self::encode_name(name);

        let version_id = DIndexVersionId::from_version_data(data);
        let version_id_string = hex::encode(version_id);

        let dirname = Path::new(&self.data_root).join("bin").join(&file_name);
        let path = dirname.join(&version_id_string);

        let file = match File::create(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(&dirname)?;
                File::create(&path)?
            }
            Err(e) => return Err(e),
        };

        zstd::stream::copy_encode(data, file, 3)?;

        Ok(version_id)
    }
}

#[cfg(test)]
mod test {
    use assert_fs::fixture::PathChild;

    use crate::blob_manager::BlobManager;

    const FILE_NAME: &str = "file.txt";

    const BLOB_VERSIONS: [[u8; 3]; 2] = [[0xFF, 0xFF, 0xFF], [0xFF, 0xFF, 0x80]];

    #[test]
    fn modify_one_blob() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let dir = tmp.child("indexes");
        let data_root: &str = &dir.path().to_string_lossy();

        let manager = BlobManager::new(data_root).unwrap();
        for version in BLOB_VERSIONS {
            let version_id = manager.insert_blob(FILE_NAME, version.to_vec()).unwrap();

            let stored_version: [u8; 3] = manager
                .get_blob_version(FILE_NAME, version_id)
                .unwrap()
                .try_into()
                .unwrap();

            assert_eq!(version, stored_version);
        }
    }

    #[test]
    fn create_one_blob() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let dir = tmp.child("indexes");
        let data_root: &str = &dir.path().to_string_lossy();

        let manager = BlobManager::new(data_root).unwrap();

        let version_id = manager
            .insert_blob(FILE_NAME, BLOB_VERSIONS[0].to_vec())
            .unwrap();

        let version: [u8; 3] = manager
            .get_blob_version(FILE_NAME, version_id)
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(BLOB_VERSIONS[0], version);
    }
}
