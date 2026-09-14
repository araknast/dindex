use std::{
    collections::HashMap,
    fs,
    io::{self, ErrorKind::NotFound},
    path::Path,
};
use thiserror::Error;

use crate::{dindex::DIndex, index_manager};

#[derive(Copy, Clone, PartialEq)]
struct DPackId(u64);

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
#[derive(Clone, PartialEq)]
struct DPackIndex {
    entries: HashMap<String, DPackId>,
    head: DPackId,
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

struct DPack {
    entries: Vec<DIndex>,
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
}

impl DPackManager {
    const INDEX_FILE_NAME: &str = "INDEX";
    const MAX_DPACK_SIZE_BYTES: u32 = 4000;
    pub fn new(path: impl AsRef<Path>) -> Result<DPackManager, DPackIndexParseError> {
        let index_path = path.as_ref().join(Self::INDEX_FILE_NAME);
        let index_data = match fs::read(index_path) {
            Ok(data) => data,
            Err(e) if e.kind() == NotFound => Vec::new(),
            Err(e) => return Err(DPackIndexParseError::FileLoad(e)),
        };

        Ok(DPackManager {
            index: DPackIndex::try_from(index_data)?,
        })
    }

    pub fn try_load(&self, name: &str) -> Result<DIndex, index_manager::LoadError> {
        Err(index_manager::LoadError::Nonexistent)
    }
    pub fn try_persist(&self, index: &DIndex) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::AlreadyExists, ""))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::dpack_manager::{DPackId, DPackIndex};

    #[test]
    fn test_serialize_deserialize_empty() {
        let index = DPackIndex {
            entries: HashMap::new(),
            head: DPackId(0),
        };

        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DPackIndex = serialized.try_into().unwrap();

        assert!(deserialized == index)
    }

    #[test]
    fn test_serialize_deserialize() {
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
