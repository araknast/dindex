use crate::dindex::DIndex;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DPack {
    entries: HashMap<String, DIndex>,
}

impl DPack {
    pub(super) fn new() -> DPack {
        DPack {
            entries: HashMap::new(),
        }
    }

    pub(super) fn insert(&mut self, index: DIndex) {
        self.entries.insert(index.name(), index);
    }

    pub(super) fn into_entry(mut self, name: &str) -> Option<DIndex> {
        self.entries.remove(name)
    }
}

impl From<Vec<u8>> for DPack {
    fn from(data: Vec<u8>) -> DPack {
        let mut entries = HashMap::new();
        let mut iter = data.into_iter();
        while let Ok(index) = DIndex::from_byte_iter(&mut iter) {
            entries.insert(index.name(), index);
        }

        DPack { entries }
    }
}

impl From<DPack> for Vec<u8> {
    fn from(pack: DPack) -> Vec<u8> {
        let mut data = Vec::new();
        for (_, entry) in pack.entries {
            data.append(&mut Vec::<u8>::from(entry));
        }
        data
    }
}
