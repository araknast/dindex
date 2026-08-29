use std::{collections::HashMap, ops::Range};

#[derive(Debug, PartialEq)]
struct DIndexRange((usize, usize));

impl From<DIndexRange> for Range<usize> {
    fn from(range: DIndexRange) -> Range<usize> {
        Range {
            start: range.0.0,
            end: range.0.1,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DIndexKey(Vec<DIndexRange>);

impl DIndexKey {
    fn into_ranges(self) -> impl Iterator<Item = DIndexRange> {
        self.0.into_iter()
    }
}

#[derive(Clone)]
pub struct DIndex {
    line_map: HashMap<String, usize>,
    lines: Vec<String>,
}

impl From<Vec<u8>> for DIndex {
    fn from(byte_array: Vec<u8>) -> DIndex {
        let mut lines = Vec::new();
        let mut line_map = HashMap::new();
        let data = String::from_utf8_lossy_owned(byte_array);
        for line in data.split_inclusive("\n") {
            if !line_map.contains_key(line) {
                line_map.insert(line.to_string(), lines.len());
                lines.push(line.to_string());
            }
        }
        DIndex { line_map, lines }
    }
}

impl From<DIndex> for Vec<u8> {
    fn from(index: DIndex) -> Vec<u8> {
        index
            .lines
            .into_iter()
            .map(String::into_bytes)
            .flatten()
            .collect()
    }
}

impl DIndex {
    pub fn new() -> DIndex {
        DIndex {
            line_map: HashMap::new(),
            lines: Vec::new(),
        }
    }

    // Takes a string containing file data, adds it to the index, and returns
    // the file's key in the index
    pub fn update(&mut self, file_data: &str) -> DIndexKey {
        let mut ranges = Vec::new();
        let mut range_start = 0;
        let mut range_end = 0;

        for line in file_data.split_inclusive("\n") {
            if !self.line_map.contains_key(line) {
                self.line_map.insert(line.to_string(), self.lines.len());
                self.lines.push(line.to_string());
            }
        }
        for line in file_data.split_inclusive("\n") {
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

    pub fn get(&mut self, key: DIndexKey) -> String {
        let mut key = key.into_ranges();
        let mut data: Vec<String> = Vec::new();
        while let Some(range) = key.next() {
            data.extend_from_slice(&self.lines[Range::from(range)]);
        }
        data.join("")
    }
}

#[cfg(test)]
mod test {
    use crate::manager::dindex::DIndex;

    const FILE1: &str = "lines\nof\nthe\nfile\n";
    const FILE2: &str = "the\nfile\n";
    const FILE3: &str = "the\nfile\nlines\nof\n";
    const FILE4: &str = "some\nnew\nlines\nof\nimportance\nfor\nthe\nfile\nhere\n";
    const FILE5: &str = "whole\ndifferent\ntext\n";

    #[test]
    fn test_serialize_deserialize() {
        let mut index = DIndex::new();
        let _ = [FILE1, FILE2, FILE3, FILE4, FILE5].map(|f| index.update(f));
        let serialized: Vec<u8> = index.clone().into();
        let deserialized: DIndex = serialized.into();
        assert!(index.lines == deserialized.lines);
        assert!(index.line_map.len() == deserialized.line_map.len());
    }
    #[test]
    fn test_get_file() {
        let mut index = DIndex::new();
        let files = [FILE1, FILE2, FILE3, FILE4, FILE5];
        for file in files {
            let key = index.update(file);
            let data = index.get(key);
            assert!(file == data, "{file:?} | {data:?}");
        }
    }

    #[test]
    fn test_update_new_file() {
        let mut index = DIndex::new();
        let key = index.update(FILE1);
        assert!(key.0.len() == 1);
    }
    #[test]
    fn test_update_subset_files() {
        let mut index = DIndex::new();
        let key1 = index.update(FILE1);
        let key2 = index.update(FILE2);
        let key3 = index.update(FILE3);

        assert!(key1.0.len() == 1);
        assert!(key2.0.len() == 2);
        assert!(key3.0.len() == 3);
    }
    #[test]
    fn test_update_intersecting_files() {
        let mut index = DIndex::new();
        let key1 = index.update(FILE1);
        let key2 = index.update(FILE4);
        assert!(key1.0.len() == 1);
        assert!(key2.0.len() == 6);
    }

    #[test]
    fn test_update_disjoint_files() {
        let mut index = DIndex::new();
        let key1 = index.update(FILE1);
        let key2 = index.update(FILE5);
        assert!(key1.0.len() == 1);
        assert!(key2.0.len() == 2);
    }
}
