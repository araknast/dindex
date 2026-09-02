use std::env;
use std::fs;

use dindex::index_manager::{DIndexManager, DIndexVersionId};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Not enough arguments.");
        std::process::exit(1)
    }
    let data_root = &args[1];
    let snap_store = format!("{}/{}", data_root, "snaps");
    let head_snap = format!("{}/{}", snap_store, "HEAD");

    let manager: SnapshotManager = SnapshotManager::new(data_root, snap_store, head_snap);
    manager.new_snap();
}
