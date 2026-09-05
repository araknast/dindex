use core::fmt;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, ops::Range};

#[derive(Debug, PartialEq, Clone, Copy)]
struct DIndexRange((usize, usize));

impl DIndexRange {
    fn into_bytes(self) -> [u8; 16] {
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

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct DIndexVersionId([u8; DIndexVersionId::LEN_BYTES]);

impl DIndexVersionId {
    const LEN_BYTES: usize = 32;
    pub fn from_version_data(data: &str) -> DIndexVersionId {
        DIndexVersionId(Sha256::digest(data).into())
    }

    fn into_bytes(self) -> [u8; DIndexVersionId::LEN_BYTES] {
        self.into()
    }
}

impl AsRef<[u8]> for DIndexVersionId {
    fn as_ref(&self) -> &[u8] {
        &self.0
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

#[derive(Clone, PartialEq, Debug)]
pub struct DIndexVersion {
    parent: DIndexVersionId,
    data_key: DIndexKey,
}

impl From<DIndexRange> for Range<usize> {
    fn from(range: DIndexRange) -> Range<usize> {
        Range {
            start: range.0.0,
            end: range.0.1,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DIndexKey(Vec<DIndexRange>);

impl DIndexKey {
    fn ranges(&self) -> impl Iterator<Item = &DIndexRange> {
        self.0.iter()
    }
    fn into_ranges(self) -> impl Iterator<Item = DIndexRange> {
        self.0.into_iter()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug)]
pub struct DeserializationError(String);

impl fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<&str> for DeserializationError {
    fn from(str: &str) -> DeserializationError {
        DeserializationError(String::from(str))
    }
}

impl std::error::Error for DeserializationError {}

#[derive(Clone)]
pub struct DIndex {
    name: String,
    version_map: HashMap<DIndexVersionId, DIndexVersion>,
    line_map: HashMap<String, usize>,
    lines: Vec<String>,
}

// Creates a DIndex from a byte array
impl TryFrom<Vec<u8>> for DIndex {
    type Error = DeserializationError;
    fn try_from(byte_array: Vec<u8>) -> Result<Self, Self::Error> {
        fn take_u64(iter: &mut impl Iterator<Item = u8>) -> Result<u64, DeserializationError> {
            Ok(u64::from_be_bytes(take_bytes(iter)?))
        }

        fn take_bytes<const N: usize>(
            iter: &mut impl Iterator<Item = u8>,
        ) -> Result<[u8; N], DeserializationError> {
            let mut arr: [u8; N] = [0; N];
            for i in 0..N {
                arr[i] = iter.next().ok_or("File ended early.")?;
            }
            Ok(arr)
        }

        fn take_name(iter: &mut impl Iterator<Item = u8>) -> Result<String, DeserializationError> {
            let mut data = Vec::new();
            while let Some(byte) = iter.next() {
                if byte == b'\n' {
                    return Ok(String::from_utf8_lossy_owned(data));
                } else {
                    data.push(byte)
                }
            }

            Err("File ended early.".into())
        }

        let mut version_map = HashMap::new();
        let mut iter = byte_array.into_iter();

        let name = take_name(&mut iter)?;
        let map_size = take_u64(&mut iter)?;

        for _ in 0..map_size {
            let version_id: [u8; DIndexVersionId::LEN_BYTES] = take_bytes(&mut iter)?;
            let parent_id: [u8; DIndexVersionId::LEN_BYTES] = take_bytes(&mut iter)?;
            let data_key_len = take_u64(&mut iter)?;

            let mut data_key_vec: Vec<DIndexRange> =
                Vec::with_capacity(data_key_len.try_into().expect("capacity > usize"));

            for _ in 0..data_key_len {
                let range_start: usize = take_u64(&mut iter)?
                    .try_into()
                    .expect("range_start > usize");
                let range_end: usize = take_u64(&mut iter)?
                    .try_into()
                    .expect("range_start > usize");
                data_key_vec.push(DIndexRange((range_start, range_end)));
            }

            let data_key = DIndexKey(data_key_vec);

            version_map.insert(
                DIndexVersionId(version_id),
                DIndexVersion {
                    parent: DIndexVersionId(parent_id),
                    data_key,
                },
            );
        }
        let mut line_map = HashMap::new();
        let mut lines = Vec::new();
        let data = String::from_utf8_lossy_owned(iter.collect());
        for line in data.split_inclusive("\n") {
            if !line_map.contains_key(line) {
                line_map.insert(line.to_string(), lines.len());
                lines.push(line.to_string());
            }
        }
        Ok(DIndex {
            name,
            version_map,
            line_map,
            lines,
        })
    }
}

// Serializes the DIndex into bytes format
impl From<DIndex> for Vec<u8> {
    fn from(index: DIndex) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let map_size = index.version_map.len() as u64;
        output.extend(index.name.as_bytes());
        output.push(b'\n');
        output.extend(map_size.to_be_bytes());

        for (version_id, value) in index.version_map {
            // id
            output.extend(version_id.into_bytes());

            //parent id
            output.extend(value.parent.into_bytes());

            // length of data key
            output.extend(
                u64::try_from(value.data_key.len())
                    .expect("usize > 64 ??")
                    .to_be_bytes(),
            );

            // data key
            for range in value.data_key.into_ranges() {
                output.extend(range.into_bytes());
            }
        }

        // index lines
        output.extend(index.lines.into_iter().map(String::into_bytes).flatten());

        output
    }
}

impl DIndex {
    pub fn new(name: &str) -> DIndex {
        DIndex {
            name: String::from(name),
            version_map: HashMap::new(),
            line_map: HashMap::new(),
            lines: Vec::new(),
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
    // Create a new version in the DIndex containing the data in version_data
    // Invariant: data with the same version_id will have the same data key for the same DIndex
    pub fn insert_version(
        &mut self,
        version_data: &str,
        parent: Option<DIndexVersionId>,
    ) -> DIndexVersionId {
        let version_id = DIndexVersionId::from_version_data(version_data);
        if parent != Some(version_id) {
            let parent = parent.unwrap_or(version_id);
            let data_key = self.key_from_data(version_data);
            self.version_map
                .insert(version_id, DIndexVersion { parent, data_key });
        }
        version_id
    }

    pub fn get_version_data(&self, id: DIndexVersionId) -> Option<String> {
        Some(self.data_from_key(&self.get_version(id)?.data_key))
    }

    pub fn get_version(&self, id: DIndexVersionId) -> Option<&DIndexVersion> {
        self.version_map.get(&id)
    }
    // Takes a string containing file data, adds it to the index, and returns
    // the file's key in the index
    fn key_from_data(&mut self, version_data: &str) -> DIndexKey {
        let mut ranges = Vec::new();
        let mut range_start = 0;
        let mut range_end = 0;

        for line in version_data.split_inclusive("\n") {
            if !self.line_map.contains_key(line) {
                self.line_map.insert(line.to_string(), self.lines.len());
                self.lines.push(line.to_string());
            }
        }
        for line in version_data.split_inclusive("\n") {
            let line_num = *self.line_map.get(line).unwrap();
            if line_num != range_end {
                ranges.push(DIndexRange((range_start, range_end)));
                range_start = line_num;
                range_end = line_num + 1;
            } else {
                range_end = line_num + 1;
            }
        }
        ranges.push(DIndexRange((range_start, range_end)));
        DIndexKey(ranges)
    }

    fn data_from_key(&self, key: &DIndexKey) -> String {
        let mut key = key.ranges();
        let mut data: Vec<String> = Vec::new();
        while let Some(range) = key.next() {
            data.extend_from_slice(&self.lines[Range::from(*range)]);
        }
        data.join("")
    }
}

#[cfg(test)]
mod test {
    use crate::dindex::DIndex;

    const FILE1: &str = "lines\nof\nthe\nfile\n";
    const FILE2: &str = "the\nfile\n";
    const FILE3: &str = "the\nfile\nlines\nof\n";
    const FILE4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const FILE5: &str = "whole\ndifferent\ntext\n";

    #[test]
    fn test_serialize_deserialize() {
        let name = "New DIndex";
        let mut index = DIndex::new(name);
        let root_version_id = index.insert_version(FILE1, None);

        let child_version_ids =
            [FILE2, FILE3, FILE4, FILE5].map(|f| index.insert_version(f, Some(root_version_id)));

        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DIndex = serialized.try_into().unwrap();

        let root_version = index.get_version(root_version_id).unwrap();
        let deserialized_root_version = deserialized.get_version(root_version_id).unwrap();
        assert!(*root_version == *deserialized_root_version);

        for child_version_id in child_version_ids {
            let child_version = index.get_version(child_version_id).unwrap();
            let deserialized_child_version = deserialized.get_version(child_version_id).unwrap();
            assert!(
                *child_version == *deserialized_child_version,
                "{child_version:?}|{deserialized_child_version:?}"
            );
        }

        assert!(index.lines == deserialized.lines);
        assert!(index.line_map.len() == deserialized.line_map.len());
        assert!(index.version_map.len() == 5);
        assert!(index.version_map.len() == deserialized.version_map.len());
        assert!(index.name == deserialized.name);
    }
    #[test]
    fn test_get_file() {
        let mut index = DIndex::new("");
        let files = [FILE1, FILE2, FILE3, FILE4, FILE5];
        for file in files {
            let key = index.key_from_data(file);
            let data = index.data_from_key(&key);
            assert!(file == data, "{file:?} | {data:?}");
        }
    }

    #[test]
    fn test_update_new_file() {
        let mut index = DIndex::new("");
        let key = index.key_from_data(FILE1);
        assert!(key.0.len() == 1);
    }
    #[test]
    fn test_update_subset_files() {
        let mut index = DIndex::new("");
        let key1 = index.key_from_data(FILE1);
        let key2 = index.key_from_data(FILE2);
        let key3 = index.key_from_data(FILE3);

        assert!(key1.0.len() == 1);
        assert!(key2.0.len() == 2);
        assert!(key3.0.len() == 3);
    }
    #[test]
    fn test_update_intersecting_files() {
        let mut index = DIndex::new("");
        let key1 = index.key_from_data(FILE1);
        let key2 = index.key_from_data(FILE4);
        assert!(key1.0.len() == 1);
        assert!(key2.0.len() == 6);
    }

    #[test]
    fn test_update_disjoint_files() {
        let mut index = DIndex::new("");
        let key1 = index.key_from_data(FILE1);
        let key2 = index.key_from_data(FILE5);
        assert!(key1.0.len() == 1);
        assert!(key2.0.len() == 2);
    }
}
