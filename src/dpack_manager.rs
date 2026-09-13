use std::{
    collections::HashMap,
    fs,
    io::{self, ErrorKind::NotFound},
    path::Path,
};
use thiserror::Error;

use crate::{dindex::DIndex, index_manager::DIndexLoadError};

#[derive(Clone, PartialEq)]
struct DPackIndex {
    entries: HashMap<String, u64>,
    head: u64,
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
                head: 0,
            });
        }

        let mut entries = HashMap::new();
        let mut iter = data.into_iter();
        let head = take_u64(&mut iter)?;
        let size = take_u64(&mut iter)?;
        for _ in 0..size {
            let name = take_name(&mut iter)?;
            let pack_id = take_u64(&mut iter)?;
            entries.insert(name, pack_id);
        }
        Ok(DPackIndex { entries, head })
    }
}

impl From<DPackIndex> for Vec<u8> {
    fn from(index: DPackIndex) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend(index.head.to_be_bytes());
        output.extend(
            u64::try_from(index.entries.len())
                .expect("usize > 64 ??")
                .to_be_bytes(),
        );
        for (name, pack_id) in index.entries {
            output.extend(name.into_bytes());
            output.push(b'\0');
            output.extend(pack_id.to_be_bytes());
        }
        output
    }
}

pub struct DPackManager {
    index: DPackIndex,
}

impl DPackManager {
    const INDEX_FILE_NAME: &str = "INDEX";
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

    pub fn try_load(&self, name: &str) -> Result<DIndex, DIndexLoadError> {
        Err(DIndexLoadError::Nonexistent)
    }
    pub fn try_persist(&self, index: &DIndex) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::AlreadyExists, ""))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::dpack_manager::DPackIndex;

    #[test]
    fn test_serialize_deserialize_empty() {
        let index = DPackIndex {
            entries: HashMap::new(),
            head: 0,
        };

        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DPackIndex = serialized.try_into().unwrap();

        assert!(deserialized == index)
    }

    #[test]
    fn test_serialize_deserialize() {
        let index = DPackIndex {
            entries: HashMap::from([
                (String::from("file1"), 0),
                (String::from("file2"), 1),
                (String::from("file3"), 2),
            ]),
            head: 2,
        };
        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DPackIndex = serialized.try_into().unwrap();

        assert!(deserialized == index)
    }

    #[test]
    fn test_empty_index() {}
}
