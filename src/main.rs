#![feature(string_from_utf8_lossy_owned)]
mod manager;
use std::env;

use crate::manager::DIndexManager;
fn main() {
    let args: Vec<String> = env::args().collect();
    let data_root = &args[1];
    let mut index: DIndexManager = DIndexManager::new(data_root);
    // index.update(&fs::read_to_string(filename).unwrap());
}
