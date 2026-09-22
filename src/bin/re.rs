use std::env;

use dindex::{dindex::DIndexVersionId, snap_manager::SnapshotManager};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Not enough arguments.");
        std::process::exit(1)
    }
    let data_root = &args[1];
    let snapshot_id_string = &args[2];
    let target_dir = &args[3];

    let manager: SnapshotManager = SnapshotManager::new(data_root).unwrap();
    let snapshot_id: DIndexVersionId = hex::decode(snapshot_id_string).unwrap().try_into().unwrap();
    manager
        .snapshot_into_dir(snapshot_id.into(), target_dir)
        .unwrap();
}
