use super::dpack_id::DPackId;
use super::errors::DPackIndexLoadError;
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
pub(super) struct DPackIndex {
    entries: HashMap<String, DPackId>,
    head: DPackId,
}

impl DPackIndex {
    pub(super) fn new(entries: HashMap<String, DPackId>, head: DPackId) -> DPackIndex {
        DPackIndex { entries, head }
    }
    pub(super) fn default() -> DPackIndex {
        DPackIndex {
            entries: HashMap::new(),
            head: DPackId::default(),
        }
    }
    pub(super) fn head(&self) -> DPackId {
        self.head
    }
    pub(super) fn get_pack_id(&self, name: &str) -> Option<DPackId> {
        self.entries.get(name).copied()
    }
    pub(super) fn insert(&mut self, name: &str, id: DPackId) {
        self.entries.insert(name.to_string(), id);
    }
    pub(super) fn increment_head(&mut self) {
        self.head = self.head.next();
    }
}

impl TryFrom<Vec<u8>> for DPackIndex {
    type Error = DPackIndexLoadError;
    fn try_from(data: Vec<u8>) -> Result<DPackIndex, Self::Error> {
        fn take_u64(iter: &mut impl Iterator<Item = u8>) -> Result<u64, DPackIndexLoadError> {
            Ok(u64::from_be_bytes(take_bytes(iter)?))
        }

        fn take_bytes<const N: usize>(
            iter: &mut impl Iterator<Item = u8>,
        ) -> Result<[u8; N], DPackIndexLoadError> {
            let mut arr: [u8; N] = [0; N];
            for i in 0..N {
                arr[i] = iter.next().ok_or(DPackIndexLoadError::EarlyTermination)?;
            }
            Ok(arr)
        }

        fn take_name(iter: &mut impl Iterator<Item = u8>) -> Result<String, DPackIndexLoadError> {
            let mut data = Vec::new();
            while let Some(byte) = iter.next() {
                if byte == b'\0' {
                    return Ok(String::from_utf8_lossy_owned(data));
                } else {
                    data.push(byte)
                }
            }

            Err(DPackIndexLoadError::EarlyTermination)
        }

        if data.len() == 0 {
            return Ok(DPackIndex {
                entries: HashMap::new(),
                head: DPackId::new(0),
            });
        }

        let mut entries = HashMap::new();
        let mut iter = data.into_iter();
        let head = DPackId::new(take_u64(&mut iter)?);
        let size = take_u64(&mut iter)?;
        for _ in 0..size {
            let name = take_name(&mut iter)?;
            let pack_id = DPackId::new(take_u64(&mut iter)?);
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
