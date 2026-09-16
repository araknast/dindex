mod dpack;
mod dpack_id;
mod dpack_index;
mod errors;

use dpack::DPack;
use dpack_id::DPackId;
use dpack_index::DPackIndex;
pub use errors::{DPackIndexLoadError, DPackLoadError, DPackPersistError};

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use crate::dindex::DIndex;

pub struct DPackManager {
    pack_dir: PathBuf,
    index_path: PathBuf,
}
impl DPackManager {
    const INDEX_FILE_NAME: &str = "index";
    const PACK_DIR_NAME: &str = "packs";
    const MAX_DPACK_SIZE_BYTES: u32 = 4000;
    pub fn new(data_root: impl AsRef<Path>) -> Result<DPackManager, DPackIndexLoadError> {
        let pack_dir = data_root.as_ref().join(Self::PACK_DIR_NAME);
        fs::create_dir_all(&pack_dir)?;

        let index_path = data_root.as_ref().join(Self::INDEX_FILE_NAME);
        match fs::exists(&index_path) {
            Ok(true) => (),
            _ => {
                let head_path = pack_dir.join(String::from(DPackId::default()));
                File::create(head_path)?;
                fs::write(&index_path, Vec::<u8>::from(DPackIndex::default()))?;
            }
        };

        Ok(DPackManager {
            pack_dir,
            index_path,
        })
    }

    fn get_head_pack(&mut self, index: &mut DPackIndex) -> io::Result<DPack> {
        let pack_path = self.pack_dir.join(String::from(index.head()));
        let data = fs::read(pack_path)?;
        if data.len()
            > Self::MAX_DPACK_SIZE_BYTES
                .try_into()
                .expect("usize < 32 ??")
        {
            index.increment_head();
            Ok(DPack::new())
        } else {
            Ok(DPack::from(data))
        }
    }

    fn load_pack(&self, id: DPackId) -> io::Result<DPack> {
        let pack_path = self.pack_dir.join(String::from(id));
        Ok(fs::read(pack_path)?.into())
    }

    fn persist_pack(&self, pack: DPack, id: DPackId) -> io::Result<()> {
        let path = self.pack_dir.join(String::from(id));
        fs::write(path, Vec::<u8>::from(pack))
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
        let mut index = self.load_index()?;
        let mut head_pack = match self.get_head_pack(&mut index) {
            Ok(pack) => pack,
            Err(e) => {
                return Err(e.into());
            }
        };

        let index_name = dindex.name();
        index.insert(&index_name, index.head());
        head_pack.insert(dindex);
        self.persist_pack(head_pack, index.head())?;
        self.persist_index(index)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        dindex::DIndex,
        dpack_manager::{DPack, DPackId, DPackIndex, DPackManager},
    };

    const VERSION1: &str = "lines\nof\nthe\nfile";
    const VERSION2: &str = "the\nfile\n";
    const VERSION3: &str = "the\nfile\nlines\nof\n";
    const VERSION4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const VERSION5: &str = "whole\ndifferent\ntext\n";

    #[test]
    fn test_load_persist() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let data_root: &str = &tmp.path().to_string_lossy();

        let file_name = "file.txt";
        let version_data = [VERSION1, VERSION2, VERSION3, VERSION4, VERSION5];

        let mut manager = DPackManager::new(data_root).unwrap();
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
