#![feature(string_from_utf8_lossy_owned)]
mod manager;
use std::{env, fs};

use crate::manager::DIndexManager;
fn main() {
    let args: Vec<String> = env::args().collect();
    let mut index: DIndexManager = DIndexManager::new();
    let filename = &args[1];
    // index.update(&fs::read_to_string(filename).unwrap());
}
