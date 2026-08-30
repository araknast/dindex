mod dindex;

use dindex::DIndex;
use dindex::DIndexKey;
// How we get string data out of the dpack
struct DataKey((usize, DIndexKey));

struct DPackObjectId(String);
// a DIndex-ed object
pub struct DPackObject {}
pub struct DPackManager {
    indexes: Vec<DIndex>,
    data_root: String,
}

impl DPackManager {
    pub fn new() -> DPackManager {
        DPackManager {
            indexes: Vec::new(),
            data_root: String::new(),
        }
    }

    pub fn load_from_path(path: String) -> DPackManager {
        DPackManager {
            indexes: Vec::new(),
            data_root: String::new(),
        }
    }
    pub fn persist() {}

    fn get_file_from_index(key: DataKey) {}
    pub fn get_file() {}
}
