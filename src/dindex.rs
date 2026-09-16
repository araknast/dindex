use std::{collections::HashMap, ops::Range};

pub use errors::DeserializationError;
use key::DIndexKey;
use range::DIndexRange;
use version::DIndexVersion;
pub use version_id::DIndexVersionId;

mod errors;
mod key;
mod range;
mod version;
mod version_id;

#[derive(Debug, Clone, PartialEq)]
pub struct DIndex {
    name: String,
    head: DIndexVersionId,
    version_map: HashMap<DIndexVersionId, DIndexVersion>,
    line_map: HashMap<String, usize>,
    lines: Vec<String>,
}

impl TryFrom<Vec<u8>> for DIndex {
    type Error = DeserializationError;
    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        DIndex::from_byte_iter(&mut data.into_iter())
    }
}

// Serializes the DIndex into bytes format
impl From<DIndex> for Vec<u8> {
    fn from(index: DIndex) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let map_size = index.version_map.len() as u64;
        output.extend(index.name.as_bytes());
        output.push(b'\0');
        output.extend(<[u8; _]>::from(index.head));
        output.extend(map_size.to_be_bytes());

        for (version_id, value) in index.version_map {
            // id
            output.extend(version_id.into_bytes());

            //prev id
            output.extend(value.prev().into_bytes());

            //next id
            output.extend(value.next().into_bytes());

            // length of data key
            output.extend(
                u64::try_from(value.key_len())
                    .expect("usize > 64 ??")
                    .to_be_bytes(),
            );

            // data key
            for range in value.into_data_key().into_ranges() {
                output.extend(range.into_bytes());
            }
        }

        // number of index lines
        output.extend(
            u64::try_from(index.lines.len())
                .expect("usize > 64 ??")
                .to_be_bytes(),
        );

        // index lines
        for line in index.lines {
            output.extend(line.into_bytes());
            output.push(b'\n');
        }

        output
    }
}

impl DIndex {
    pub fn new(name: &str, data: &str) -> DIndex {
        let mut index = DIndex {
            name: String::from(name),
            head: DIndexVersionId::default(),
            version_map: HashMap::new(),
            line_map: HashMap::from([(String::from(""), 0)]),
            lines: vec![String::from("")],
        };

        index.head = DIndexVersionId::from_version_data(data);
        let data_key = index.key_from_data(data);
        index.version_map.insert(
            index.head,
            DIndexVersion::new(index.head, index.head, data_key),
        );
        index
    }

    pub fn from_byte_iter(
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<DIndex, DeserializationError> {
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

        fn take_line(iter: &mut impl Iterator<Item = u8>) -> String {
            let mut data = Vec::new();
            while let Some(byte) = iter.next() {
                if byte == b'\n' {
                    break;
                } else {
                    data.push(byte);
                }
            }
            String::from_utf8_lossy_owned(data)
        }

        fn take_name(iter: &mut impl Iterator<Item = u8>) -> Result<String, DeserializationError> {
            let mut data = Vec::new();
            while let Some(byte) = iter.next() {
                if byte == b'\0' {
                    return Ok(String::from_utf8_lossy_owned(data));
                } else {
                    data.push(byte)
                }
            }

            Err("File ended early.".into())
        }

        let mut version_map = HashMap::new();

        let name = take_name(iter)?;
        let head: DIndexVersionId = take_bytes(iter)?.into();
        let map_size = take_u64(iter)?;

        for _ in 0..map_size {
            let version_id: DIndexVersionId = take_bytes::<_>(iter)?.into();
            let prev_id: DIndexVersionId = take_bytes::<_>(iter)?.into();
            let next_id: DIndexVersionId = take_bytes::<_>(iter)?.into();
            let data_key_len = take_u64(iter)?;

            let mut data_key_vec: Vec<DIndexRange> =
                Vec::with_capacity(data_key_len.try_into().expect("capacity > usize"));

            for _ in 0..data_key_len {
                let range_start: usize = take_u64(iter)?.try_into().expect("range_start > usize");
                let range_end: usize = take_u64(iter)?.try_into().expect("range_start > usize");
                data_key_vec.push(DIndexRange::new(range_start, range_end));
            }

            let data_key: DIndexKey = data_key_vec.into();

            version_map.insert(version_id, DIndexVersion::new(prev_id, next_id, data_key));
        }
        let num_lines: usize = take_u64(iter)?.try_into().expect("num lines > usize !");
        let mut line_map = HashMap::new();
        let mut lines = Vec::with_capacity(num_lines);

        for _ in 0..num_lines {
            let line = take_line(iter);
            if !line_map.contains_key(&line) {
                line_map.insert(line.to_string(), lines.len());
                lines.push(line.to_string());
            }
        }
        Ok(DIndex {
            name,
            head,
            version_map,
            line_map,
            lines,
        })
    }

    pub fn head(&self) -> DIndexVersionId {
        self.head
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    // Create a new version in the DIndex containing the data in version_data
    // Invariant: data with the same version_id will have the same data key for the same DIndex
    pub fn insert_version(&mut self, version_data: &str) -> DIndexVersionId {
        let version_id = DIndexVersionId::from_version_data(version_data);
        let data_key = self.key_from_data(version_data);
        self.version_map
            .entry(version_id)
            .or_insert(DIndexVersion::new(self.head, version_id, data_key));

        self.update_head(version_id);

        version_id
    }

    fn update_head(&mut self, new_head: DIndexVersionId) {
        let prev_head = self
            .version_map
            .get_mut(&self.head)
            .expect("DIndex has no head!");
        prev_head.set_next(new_head);
        self.head = new_head;
    }

    pub fn get_version_data(&self, id: DIndexVersionId) -> Option<String> {
        Some(self.data_from_key(self.get_version(id)?.data_key()))
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

        for line in version_data.split("\n") {
            if !self.line_map.contains_key(line) {
                self.line_map.insert(line.to_string(), self.lines.len());
                self.lines.push(line.to_string());
            }
        }
        for line in version_data.split("\n") {
            let line_num = *self.line_map.get(line).unwrap();
            if line_num == range_end {
                range_end = line_num + 1;
                continue;
            }

            if range_start != range_end {
                ranges.push(DIndexRange::new(range_start, range_end));
            }

            range_start = line_num;
            range_end = line_num + 1;
        }
        ranges.push(DIndexRange::new(range_start, range_end));
        ranges.into()
    }

    fn data_from_key(&self, key: &DIndexKey) -> String {
        let mut key = key.ranges();
        let mut data: Vec<String> = Vec::new();
        while let Some(range) = key.next() {
            data.extend_from_slice(&self.lines[Range::from(*range)]);
        }
        data.join("\n")
    }
}

#[cfg(test)]
mod test {
    use crate::dindex::DIndex;

    const VERSION1: &str = "lines\nof\nthe\nfile";
    const VERSION2: &str = "the\nfile\n";
    const VERSION3: &str = "the\nfile\nlines\nof\n";
    const VERSION4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const VERSION5: &str = "whole\ndifferent\ntext\n";

    #[test]
    fn test_serialize_deserialize() {
        let name = "New DIndex";
        let mut index = DIndex::new(name, VERSION1);
        let root_version_id = index.head;

        let child_version_ids =
            [VERSION2, VERSION3, VERSION4, VERSION5].map(|f| index.insert_version(f));

        let serialized: Vec<u8> = index.clone().into();
        let deserialized = DIndex::try_from(serialized).unwrap();

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
        assert!(index.head == deserialized.head);
        assert!(index == deserialized);
    }

    #[test]
    fn test_prev_and_next() {
        let mut index = DIndex::new("", VERSION1);
        let files = [VERSION1, VERSION2, VERSION3];
        for file in files {
            index.insert_version(file);
        }

        let serialized: Vec<u8> = index.clone().into();
        let index = DIndex::try_from(serialized).unwrap();

        let curr = index.head();
        assert!(index.get_version_data(curr).unwrap() == VERSION3);
        let prev_version = index.get_version(curr).unwrap().prev();
        let next_version = index.get_version(curr).unwrap().next();
        assert!(index.get_version_data(prev_version).unwrap() == VERSION2);
        assert!(index.get_version_data(next_version).unwrap() == VERSION3);

        let curr = prev_version;
        assert!(index.get_version_data(curr).unwrap() == VERSION2);
        let prev_version = index.get_version(curr).unwrap().prev();
        let next_version = index.get_version(curr).unwrap().next();
        assert!(index.get_version_data(prev_version).unwrap() == VERSION1);
        assert!(index.get_version_data(next_version).unwrap() == VERSION3);

        let curr = prev_version;
        assert!(index.get_version_data(curr).unwrap() == VERSION1);
        let prev_version = index.get_version(curr).unwrap().prev();
        let next_version = index.get_version(curr).unwrap().next();
        assert!(index.get_version_data(prev_version).unwrap() == VERSION1);
        assert!(index.get_version_data(next_version).unwrap() == VERSION2);
    }

    #[test]
    fn test_update_head() {
        let mut index = DIndex::new("", VERSION1);
        let root_head = index.head;
        let new_version_id = index.insert_version(VERSION2);
        let new_head = index.head;
        assert!(root_head != new_head);
        assert!(new_head == new_version_id)
    }

    #[test]
    fn test_get_file() {
        let mut index = DIndex::new("", VERSION1);
        let files = [VERSION1, VERSION2, VERSION3, VERSION4, VERSION5];
        for file in files {
            let key = index.key_from_data(file);
            let data = index.data_from_key(&key);
            assert!(file == data, "{file:?} | {data:?}");
        }
    }
}
