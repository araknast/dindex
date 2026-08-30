use sha2::{Digest, Sha256};

use super::{DIndex, DIndexKey};
use std::collections::HashMap;
struct DPackKey((usize, DIndexKey));
struct DPackObjectId([u8; DPackObjectId::LEN_BYTES]);

impl DPackObjectId {
    const LEN_BYTES: usize = 32;
    fn from_object_data(data: &str) -> DPackObjectId {
        DPackObjectId(Sha256::digest(data).into())
    }

    fn into_bytes(self) -> [u8; DPackObjectId::LEN_BYTES] {
        self.into()
    }
}

impl From<DPackObjectId> for [u8; DPackObjectId::LEN_BYTES] {
    fn from(id: DPackObjectId) -> [u8; DPackObjectId::LEN_BYTES] {
        id.0
    }
}
struct DPack {
    memebers: Vec<DIndex>,
    object_map: HashMap<DPackObjectId, DPackKey>,
}
