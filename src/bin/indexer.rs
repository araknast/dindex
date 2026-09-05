use std::env;
use std::fs;

use dindex::index_manager::{DIndexManager, DIndexVersionId};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Not enough arguments.");
        std::process::exit(1)
    }
    let operation = &args[1];
    let file_path = &args[2];
    let data_root = &args[3];

    let index: DIndexManager = DIndexManager::new(data_root);
    index.create_data_root().unwrap();

    match operation.as_str() {
        "put" => {
            let file_data: String = fs::read_to_string(file_path).unwrap();
            let version_id = index.insert(file_path, &file_data).unwrap();
            let version_id_str = hex::encode(<[u8; 32]>::from(version_id));
            println!("{version_id_str}");
        }
        "get" => {
            let version_id: DIndexVersionId = if args.len() > 3 {
                TryInto::<[u8; 32]>::try_into(hex::decode(&args[4]).unwrap())
                    .unwrap()
                    .into()
            } else {
                println!("No version id provided to get operation");
                std::process::exit(1)
            };

            let file_data = index.get_version(file_path, version_id).unwrap().unwrap();
            println!("{file_data}");
        }
        _ => {
            println!("Invalid operation");
        }
    }
}
