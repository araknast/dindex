mod dpack;
mod dpack_id;
mod dpack_index;
mod errors;

use dpack::DPack;
use dpack_id::DPackId;
use dpack_index::DPackIndex;
use errors::GetHeadError;
pub use errors::{DPackIndexLoadError, DPackLoadError, DPackPersistError, InitializationError};

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use crate::dindex::DIndex;

pub struct DPackManagerConfig {
    index_file_name: String,
    pack_dir_name: String,
    max_dpack_size_bytes: u32,
    zstd_compression_level: i32,
}

pub struct DPackManager {
    pack_dir: PathBuf,
    index_path: PathBuf,
    config: DPackManagerConfig,
}

impl DPackManager {
    pub fn new(data_root: impl AsRef<Path>) -> Result<DPackManager, InitializationError> {
        let config = DPackManagerConfig {
            index_file_name: String::from("dpack_index"),
            pack_dir_name: String::from("dpacks"),
            max_dpack_size_bytes: 4000,
            zstd_compression_level: 3,
        };

        let pack_dir = data_root.as_ref().join(&config.pack_dir_name);
        fs::create_dir_all(&pack_dir)?;

        let index_path = data_root.as_ref().join(&config.index_file_name);
        match fs::exists(&index_path) {
            Ok(true) => (),
            _ => {
                let head_path = pack_dir.join(String::from(DPackId::default()));
                let head_file = File::create(&head_path)?;
                let head_data: &[u8] = &[];
                zstd::stream::copy_encode(head_data, head_file, config.zstd_compression_level)?;
                fs::write(&index_path, Vec::<u8>::from(DPackIndex::default()))?;
            }
        };

        Ok(DPackManager {
            pack_dir,
            index_path,
            config,
        })
    }

    fn get_head_pack(&mut self, index: &mut DPackIndex) -> Result<DPack, GetHeadError> {
        let pack_path = self.pack_dir.join(String::from(index.head()));
        let data = fs::read(pack_path)?;
        if data.len()
            > self
                .config
                .max_dpack_size_bytes
                .try_into()
                .expect("usize < 32 ??")
        {
            index.increment_head();
            Ok(DPack::new())
        } else {
            Ok(self.load_pack(index.head())?)
        }
    }

    fn load_pack(&self, id: DPackId) -> Result<DPack, DPackLoadError> {
        let pack_path = self.pack_dir.join(String::from(id));
        let pack_file = File::open(pack_path)?;
        let mut pack_data = Vec::new();
        zstd::stream::copy_decode(pack_file, &mut pack_data)?;
        Ok(DPack::from(pack_data))
    }

    fn persist_pack(&self, pack: DPack, id: DPackId) -> Result<(), DPackPersistError> {
        let pack_path = self.pack_dir.join(String::from(id));
        let pack_data: &[u8] = &Vec::<u8>::from(pack);
        let pack_file = File::create(pack_path)?;
        zstd::stream::copy_encode(pack_data, pack_file, self.config.zstd_compression_level)?;
        Ok(())
    }

    fn load_index(&self) -> Result<DPackIndex, DPackIndexLoadError> {
        fs::read(&self.index_path)?.try_into()
    }

    fn persist_index(&self, index: DPackIndex) -> io::Result<()> {
        fs::write(&self.index_path, Vec::<u8>::from(index))
    }

    pub fn try_load(&self, name: &str) -> Result<Option<DIndex>, DPackLoadError> {
        let index = self.load_index()?;
        let pack_id = match index.get_pack_id(name) {
            Some(id) => id,
            None => return Ok(None),
        };

        let pack = self.load_pack(pack_id)?;

        Ok(Some(
            pack.into_entry(name)
                .expect("DIndex does not exist in its mapped DPack!"),
        ))
    }

    pub fn try_persist(&mut self, dindex: DIndex) -> Result<(), DPackPersistError> {
        let mut pack_index = self.load_index()?;
        let (mut pack, pack_id) = match pack_index.get_pack_id(&dindex.name()) {
            Some(id) => (self.load_pack(id)?, id),
            None => (self.get_head_pack(&mut pack_index)?, pack_index.head()),
        };

        pack_index.insert(&dindex.name(), pack_id);
        pack.insert(dindex);
        self.persist_pack(pack, pack_id)?;
        self.persist_index(pack_index)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, fs};

    use crate::{
        dindex::DIndex,
        dpack_manager::{DPack, DPackId, DPackIndex, DPackManager},
    };

    const VERSION1: &str = "lines\nof\nthe\nfile";
    const VERSION2: &str = "the\nfile\n";
    const VERSION3: &str = "the\nfile\nlines\nof\n";
    const VERSION4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const VERSION5: &str = "whole\ndifferent\ntext\n";

    const EMPTY_DPACK_SIZE: u32 = 9; // Size of an empy zstd compressed file

    #[test]
    fn test_load_persist_same_file_same_pack() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let data_root: &str = &tmp.path().to_string_lossy();

        let file_name = "file.txt";
        let version_data = [VERSION1, VERSION2, VERSION3, VERSION4, VERSION5];

        let mut manager = DPackManager::new(data_root).unwrap();
        manager.config.max_dpack_size_bytes = EMPTY_DPACK_SIZE;
        let mut index = DIndex::new(file_name, VERSION1);
        manager.try_persist(index.clone()).unwrap();
        let persisted = manager.try_load(file_name).unwrap().unwrap();

        assert!(index == persisted);

        for version in version_data {
            index.insert_version(version);
            manager.try_persist(index.clone()).unwrap();
            let persisted = manager.try_load(file_name).unwrap().unwrap();

            assert!(index == persisted);
        }
        let dirents: Vec<_> = fs::read_dir(manager.pack_dir)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(dirents.len() == 1);
    }

    #[test]
    fn test_load_persist_different_files_different_packs() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let data_root: &str = &tmp.path().to_string_lossy();

        let file_names = ["1", "2", "3", "4", "5"];
        let version_data = [VERSION1, VERSION2, VERSION3, VERSION4, VERSION5];

        let mut manager = DPackManager::new(data_root).unwrap();
        manager.config.max_dpack_size_bytes = EMPTY_DPACK_SIZE;
        let index = DIndex::new(file_names[0], VERSION1);
        manager.try_persist(index.clone()).unwrap();
        let persisted = manager.try_load(file_names[0]).unwrap().unwrap();

        assert!(index == persisted);

        for i in 0..version_data.len() {
            let version = version_data[i];
            let file_name = file_names[i];
            let index = DIndex::new(file_name, version);
            manager.try_persist(index.clone()).unwrap();
            let persisted = manager.try_load(file_name).unwrap().unwrap();

            assert!(index == persisted);
        }

        let dirents: Vec<_> = fs::read_dir(manager.pack_dir)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(dirents.len() == 5);
    }

    #[test]
    fn test_serialize_deserialize_dpack_single() {
        let file_name = "file.txt";
        let base = DIndex::new(file_name, VERSION1);
        let mut pack = DPack::new();
        pack.insert(base.clone());
        let serialized: Vec<u8> = pack.clone().into();
        let persisted: DPack = serialized.into();

        assert!(pack == persisted);
    }
    #[test]
    fn test_serialize_deserialize_dpack_multi() {
        let mut pack = DPack::new();

        let version_data = [VERSION1, VERSION2, VERSION3, VERSION4, VERSION5];
        for version in version_data {
            pack.insert(DIndex::new("", version));
        }

        let serialized: Vec<u8> = pack.clone().into();
        let persisted: DPack = serialized.into();

        assert!(pack == persisted);
    }

    #[test]
    fn test_serialize_deserialize_index_empty() {
        let index = DPackIndex::new(HashMap::new(), DPackId::new(0));

        let serialized: Vec<u8> = index.clone().into();
        let persisted: DPackIndex = serialized.try_into().unwrap();

        assert!(persisted == index)
    }

    #[test]
    fn test_serialize_deserialize_index() {
        let index = DPackIndex::new(
            HashMap::from([
                (String::from("file1"), DPackId::new(0)),
                (String::from("file2"), DPackId::new(1)),
                (String::from("file3"), DPackId::new(2)),
            ]),
            DPackId::new(2),
        );
        let serialized: Vec<u8> = index.clone().into();
        let persisted: DPackIndex = serialized.try_into().unwrap();

        assert!(persisted == index)
    }
}
