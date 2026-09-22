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

    let mut manager: SnapshotManager = SnapshotManager::new(data_root).unwrap();
    let snap_id = manager
        .snapshot_from_dir(target_dir, vec![".git", data_root])
        .unwrap();
    let snap_id_str = hex::encode(snap_id);
    println!("{snap_id_str}");
}
