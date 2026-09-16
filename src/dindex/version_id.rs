use sha2::{Digest, Sha256};

use super::errors::DeserializationError;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct DIndexVersionId([u8; DIndexVersionId::LEN_BYTES]);

impl DIndexVersionId {
    const LEN_BYTES: usize = 32;
    pub(super) fn default() -> DIndexVersionId {
        DIndexVersionId([0; Self::LEN_BYTES])
    }
    pub fn from_version_data(data: impl AsRef<[u8]>) -> DIndexVersionId {
        DIndexVersionId(Sha256::digest(data).into())
    }

    pub(super) fn into_bytes(self) -> [u8; Self::LEN_BYTES] {
        self.into()
    }
}

impl AsRef<[u8]> for DIndexVersionId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for DIndexVersionId {
    type Error = DeserializationError;
    fn try_from(vec: Vec<u8>) -> Result<DIndexVersionId, DeserializationError> {
        let bytes: [u8; DIndexVersionId::LEN_BYTES] = vec
            .try_into()
            .map_err(|_| DeserializationError::new("Could not deserialize version id"))?;
        Ok(DIndexVersionId(bytes))
    }
}

impl From<DIndexVersionId> for [u8; DIndexVersionId::LEN_BYTES] {
    fn from(id: DIndexVersionId) -> [u8; DIndexVersionId::LEN_BYTES] {
        id.0
    }
}

impl From<[u8; DIndexVersionId::LEN_BYTES]> for DIndexVersionId {
    fn from(arr: [u8; DIndexVersionId::LEN_BYTES]) -> DIndexVersionId {
        DIndexVersionId(arr)
    }
}
