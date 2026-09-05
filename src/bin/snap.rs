use std::env;

use dindex::snap_manager::SnapshotManager;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Not enough arguments.");
        std::process::exit(1)
    }
    let target_dir = &args[1];
    let data_root = &args[2];

    let manager: SnapshotManager = SnapshotManager::new(data_root);
    let _ = manager.init_dirs();
    let _ = manager.snapshot_from_dir(target_dir);
}
