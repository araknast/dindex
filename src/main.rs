use std::env;

use dindex::snap_manager::SnapshotManager;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Not enough arguments.");
        std::process::exit(1)
    }
    let data_root = &args[1];

    let manager: SnapshotManager = SnapshotManager::new(data_root);
    // manager.new_snap();
}
