use std::collections::HashMap;

use crate::manager::dindex::{DIndex, DIndexKey};

mod dindex;
mod dpack;

pub struct DPackManager {
    entries: HashMap<String, DIndex>,
    data_root: String,
}

impl DPackManager {
    pub fn new() -> DPackManager {
        DPackManager {
            entries: HashMap::new(),
            data_root: String::new(),
        }
    }

    fn get(name: &str, key: DIndexKey) {}
    fn insert(name: &str, data: String) {}
}
