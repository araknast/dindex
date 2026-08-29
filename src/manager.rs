mod dindex;
use std::collections::HashMap;

use dindex::DIndex;
use dindex::DIndexKey;
pub struct Key(String);
pub struct DIndexManager {
    indexes: Vec<DIndex>,
    file_map: HashMap<String, DIndexKey>,
    data_root: String,
}

impl DIndexManager {
    pub fn new() -> DIndexManager {
        DIndexManager {
            indexes: Vec::new(),
            file_map: HashMap::new(),
            data_root: String::new(),
        }
    }

    pub fn load_from_path(path: String) -> DIndexManager {
        DIndexManager {
            indexes: Vec::new(),
            file_map: HashMap::new(),
            data_root: String::new(),
        }
    }
    pub fn persist() {}
    pub fn add_file(file: String) {}

    pub fn get_file(key: Key) {}
}
