use super::range::DIndexRange;

#[derive(Debug, PartialEq, Clone)]
pub struct DIndexKey(Vec<DIndexRange>);

impl From<Vec<DIndexRange>> for DIndexKey {
    fn from(ranges: Vec<DIndexRange>) -> DIndexKey {
        DIndexKey(ranges)
    }
}

impl DIndexKey {
    pub fn ranges(&self) -> impl Iterator<Item = &DIndexRange> {
        self.0.iter()
    }
    pub fn into_ranges(self) -> impl Iterator<Item = DIndexRange> {
        self.0.into_iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
