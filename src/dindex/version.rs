use super::key::DIndexKey;
use super::version_id::DIndexVersionId;

#[derive(Clone, PartialEq, Debug)]
pub struct DIndexVersion {
    prev: DIndexVersionId,
    next: DIndexVersionId,
    data_key: DIndexKey,
}

impl DIndexVersion {
    pub(super) fn new(
        prev: DIndexVersionId,
        next: DIndexVersionId,
        data_key: DIndexKey,
    ) -> DIndexVersion {
        DIndexVersion {
            prev,
            next,
            data_key,
        }
    }

    pub(super) fn prev(&self) -> DIndexVersionId {
        self.prev
    }

    pub(super) fn next(&self) -> DIndexVersionId {
        self.next
    }

    pub(super) fn set_next(&mut self, next: DIndexVersionId) {
        self.next = next;
    }

    pub(super) fn key_len(&self) -> usize {
        self.data_key.len()
    }

    pub(super) fn into_data_key(self) -> DIndexKey {
        self.data_key
    }

    pub(super) fn data_key(&self) -> &DIndexKey {
        &self.data_key
    }
}
