use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, ErrorKind::NotFound},
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::dindex::DIndex;

#[derive(Copy, Clone, PartialEq)]
struct DPackId(u64);
impl DPackId {
    fn default() -> DPackId {
        DPackId(0)
    }
}

impl From<u64> for DPackId {
    fn from(i: u64) -> DPackId {
        DPackId(i)
    }
}

impl From<DPackId> for [u8; 8] {
    fn from(id: DPackId) -> [u8; 8] {
        id.0.to_be_bytes()
    }
}

impl From<DPackId> for String {
    fn from(id: DPackId) -> String {
        id.0.to_string()
    }
}

#[derive(Clone, PartialEq)]
struct DPackIndex {
    entries: HashMap<String, DPackId>,
    head: DPackId,
}

impl DPackIndex {
    fn new() -> DPackIndex {
        DPackIndex {
            entries: HashMap::new(),
            head: DPackId::default(),
        }
    }
    fn get_pack_id(&self, name: &str) -> Option<DPackId> {
        self.entries.get(name).copied()
    }
    fn insert(&mut self, name: &str, id: DPackId) {
        self.entries.insert(name.to_string(), id);
    }
    fn increment_head(&mut self) {
        self.head.0 += 1;
    }
}

#[derive(Debug, Error)]
#[error("Failed to persist DPack: {source}")]
pub struct DPackPersistError {
    pub index: DIndex,
    pub source: io::Error,
}

#[derive(Debug, Error)]
pub enum DPackIndexParseError {
    #[error("File ended early.")]
    EarlyTermination,
    #[error("Could not read index file")]
    FileLoad(#[from] std::io::Error),
}

impl TryFrom<Vec<u8>> for DPackIndex {
    type Error = DPackIndexParseError;
    fn try_from(data: Vec<u8>) -> Result<DPackIndex, Self::Error> {
        fn take_u64(iter: &mut impl Iterator<Item = u8>) -> Result<u64, DPackIndexParseError> {
            Ok(u64::from_be_bytes(take_bytes(iter)?))
        }

        fn take_bytes<const N: usize>(
            iter: &mut impl Iterator<Item = u8>,
        ) -> Result<[u8; N], DPackIndexParseError> {
            let mut arr: [u8; N] = [0; N];
            for i in 0..N {
                arr[i] = iter.next().ok_or(DPackIndexParseError::EarlyTermination)?;
            }
            Ok(arr)
        }

        fn take_name(iter: &mut impl Iterator<Item = u8>) -> Result<String, DPackIndexParseError> {
            let mut data = Vec::new();
            while let Some(byte) = iter.next() {
                if byte == b'\0' {
                    return Ok(String::from_utf8_lossy_owned(data));
                } else {
                    data.push(byte)
                }
            }

            Err(DPackIndexParseError::EarlyTermination)
        }

        if data.len() == 0 {
            return Ok(DPackIndex {
                entries: HashMap::new(),
                head: DPackId(0),
            });
        }

        let mut entries = HashMap::new();
        let mut iter = data.into_iter();
        let head = DPackId(take_u64(&mut iter)?);
        let size = take_u64(&mut iter)?;
        for _ in 0..size {
            let name = take_name(&mut iter)?;
            let pack_id = DPackId(take_u64(&mut iter)?);
            entries.insert(name, pack_id);
        }
        Ok(DPackIndex { entries, head })
    }
}

impl From<DPackIndex> for Vec<u8> {
    fn from(index: DPackIndex) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend(<[u8; 8]>::from(index.head));
        output.extend(
            u64::try_from(index.entries.len())
                .expect("usize > 64 ??")
                .to_be_bytes(),
        );
        for (name, pack_id) in index.entries {
            output.extend(name.into_bytes());
            output.push(b'\0');
            output.extend(<[u8; 8]>::from(pack_id));
        }
        output
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DPack {
    entries: Vec<DIndex>,
}

impl DPack {
    fn new() -> DPack {
        DPack {
            entries: Vec::new(),
        }
    }

    fn push(&mut self, index: DIndex) {
        self.entries.push(index);
    }
    fn pop(&mut self) -> Option<DIndex> {
        self.entries.pop()
    }
}

impl From<Vec<u8>> for DPack {
    fn from(data: Vec<u8>) -> DPack {
        let mut entries = Vec::new();
        let mut iter = data.into_iter();
        while let Ok(index) = DIndex::from_byte_iter(&mut iter) {
            entries.push(index)
        }

        DPack { entries }
    }
}

impl From<DPack> for Vec<u8> {
    fn from(pack: DPack) -> Vec<u8> {
        let mut data = Vec::new();
        for entry in pack.entries {
            data.append(&mut Vec::<u8>::from(entry));
        }
        data
    }
}

pub struct DPackManager {
    index: DPackIndex,
    pack_dir: PathBuf,
}
impl DPackManager {
    const INDEX_FILE_NAME: &str = "index";
    const PACK_DIR_NAME: &str = "packs";
    const MAX_DPACK_SIZE_BYTES: u32 = 4000;
    pub fn new(data_root: impl AsRef<Path>) -> Result<DPackManager, DPackIndexParseError> {
        let pack_dir = data_root.as_ref().join(Self::PACK_DIR_NAME);
        fs::create_dir_all(&pack_dir)?;

        let index_path = data_root.as_ref().join(Self::INDEX_FILE_NAME);
        let index = match fs::read(index_path) {
            Ok(data) => DPackIndex::try_from(data)?,
            Err(e) if e.kind() == NotFound => {
                let head_path = pack_dir.join(String::from(DPackId::default()));
                File::create(head_path)?;
                DPackIndex::new()
            }
            Err(e) => return Err(e.into()),
        };

        Ok(DPackManager { pack_dir, index })
    }

    fn new_head_pack(&mut self) -> DPack {
        self.index.increment_head();
        DPack::new()
    }

    fn get_head_pack(&mut self) -> io::Result<DPack> {
        let pack_path = self.pack_dir.join(String::from(self.index.head));
        let data = fs::read(pack_path)?;
        if data.len()
            > Self::MAX_DPACK_SIZE_BYTES
                .try_into()
                .expect("usize < 32 ??")
        {
            Ok(self.new_head_pack())
        } else {
            Ok(DPack::from(data))
        }
    }

    fn get_pack(&self, id: DPackId) -> io::Result<DPack> {
        let pack_path = self.pack_dir.join(String::from(id));
        Ok(fs::read(pack_path)?.into())
    }

    pub fn try_load(&self, name: &str) -> Result<Option<DIndex>, io::Error> {
        let pack_id = match self.index.get_pack_id(name) {
            Some(id) => id,
            None => return Ok(None),
        };

        let pack = self.get_pack(pack_id)?;

        for entry in pack.entries {
            if entry.name() == name {
                return Ok(Some(entry));
            }
        }
        panic!("DIndex does not exist in its DPack!")
    }

    // Returns the passed DIndex on failure, else None
    pub fn try_persist(&mut self, dindex: DIndex) -> Result<(), DPackPersistError> {
        let pack_path = self.pack_dir.join(String::from(self.index.head));
        let mut head_pack = match self.get_head_pack() {
            Ok(pack) => pack,
            Err(e) => {
                return Err(DPackPersistError {
                    index: dindex,
                    source: e,
                });
            }
        };
        self.index.insert(&dindex.name(), self.index.head);
        head_pack.push(dindex);
        let pack_data: Vec<u8> = head_pack.into();

        match fs::write(pack_path, &pack_data) {
            Ok(_) => Ok(()),
            Err(e) => {
                let dindex = DPack::try_from(pack_data)
                    .expect("Could not reserialize DPack!")
                    .pop()
                    .expect("DIndex no longer exists in pack!");

                Err(DPackPersistError {
                    index: dindex,
                    source: e,
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        dindex::DIndex,
        dpack_manager::{DPack, DPackId, DPackIndex, DPackManager},
    };

    const VERSION1: &str = "lines\nof\nthe\nfile\n";
    const VERSION2: &str = "the\nfile\n";
    const VERSION3: &str = "the\nfile\nlines\nof\n";
    const VERSION4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const VERSION5: &str = "whole\ndifferent\ntext\n";

    #[test]
    fn test_load_persist() {
        let tmp = assert_fs::TempDir::new().unwrap();
        let data_root: &str = &tmp.path().to_string_lossy();

        let file_name = "file.txt";

        let mut manager = DPackManager::new(data_root).unwrap();
        let mut base = DIndex::new(file_name, VERSION1);
        manager.try_persist(base.clone()).unwrap();
        let persisted = manager.try_load(file_name).unwrap().unwrap();

        assert!(base == persisted);

        base.insert_version(VERSION2);
        manager.try_persist(base.clone()).unwrap();
        let persisted = manager.try_load(file_name).unwrap().unwrap();
        println!("{base:#?} | {persisted:#?}");
        assert!(base == persisted);
    }

    #[test]
    fn test_serialize_deserialize_dpack_single() {
        let file_name = "file.txt";
        let base = DIndex::new(file_name, VERSION1);
        let mut pack = DPack::new();
        pack.push(base.clone());
        let serialized: Vec<u8> = pack.clone().into();
        let deserialized: DPack = serialized.into();

        assert!(pack == deserialized);
    }
    #[test]
    fn test_serialize_deserialize_dpack_multi() {
        let mut pack = DPack::new();

        let version_data = [VERSION1, VERSION2, VERSION3, VERSION4, VERSION5];
        for version in version_data {
            pack.push(DIndex::new("", version));
        }

        let serialized: Vec<u8> = pack.clone().into();
        let deserialized: DPack = serialized.into();

        assert!(pack == deserialized);
    }

    #[test]
    fn test_serialize_deserialize_index_empty() {
        let index = DPackIndex {
            entries: HashMap::new(),
            head: DPackId(0),
        };

        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DPackIndex = serialized.try_into().unwrap();

        assert!(deserialized == index)
    }

    #[test]
    fn test_serialize_deserialize_index() {
        let index = DPackIndex {
            entries: HashMap::from([
                (String::from("file1"), DPackId(0)),
                (String::from("file2"), DPackId(1)),
                (String::from("file3"), DPackId(2)),
            ]),
            head: DPackId(2),
        };
        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DPackIndex = serialized.try_into().unwrap();

        assert!(deserialized == index)
    }
}
