#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DPackId(u64);
impl DPackId {
    pub fn new(id: u64) -> DPackId {
        DPackId(id)
    }
    pub fn default() -> DPackId {
        DPackId(0)
    }
    pub fn next(&self) -> DPackId {
        DPackId(self.0 + 1)
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
