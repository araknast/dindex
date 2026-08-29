mod dindex;
use std::{env, fs};

use crate::dindex::DIndex;
fn main() {
    let args: Vec<String> = env::args().collect();
    let mut index: DIndex = DIndex::new();
    let filename = &args[1];
    // index.update(&fs::read_to_string(filename).unwrap());
}
