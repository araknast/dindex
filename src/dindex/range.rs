use std::ops::Range;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) struct DIndexRange((usize, usize));

impl DIndexRange {
    pub(super) fn new(start: usize, end: usize) -> DIndexRange {
        DIndexRange((start, end))
    }

    pub(super) fn into_bytes(self) -> [u8; 16] {
        self.into()
    }
}

// Turn a DIndexRange into a byte array representing 2 64-bit unsigned integers
impl From<DIndexRange> for [u8; 16] {
    fn from(range: DIndexRange) -> [u8; 16] {
        let mut arr = [0; 16];
        arr[..8].copy_from_slice(
            &u64::try_from(range.0.0)
                .expect("usize > 64 ??")
                .to_be_bytes(),
        );
        arr[8..].copy_from_slice(
            &u64::try_from(range.0.1)
                .expect("usize > 64 ??")
                .to_be_bytes(),
        );

        arr
    }
}

impl From<DIndexRange> for Range<usize> {
    fn from(range: DIndexRange) -> Range<usize> {
        Range {
            start: range.0.0,
            end: range.0.1,
        }
    }
}
