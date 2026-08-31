#![feature(string_from_utf8_lossy_owned)]
mod manager;
use std::env;
use std::fs;

use crate::manager::DIndexManager;
use crate::manager::dindex::DIndexVersionId;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Not enough arguments.");
        std::process::exit(1)
    }
    let operation = &args[1];
    let file_path = &args[2];
    let data_root = "./data";

    let index: DIndexManager = DIndexManager::new(data_root);
    index.create_data_root().unwrap();

    match operation.as_str() {
        "put" => {
            let file_data: String = fs::read_to_string(file_path).unwrap();
            let parent_id = if args.len() > 3 {
                TryInto::<[u8; 32]>::try_into(hex::decode(&args[2]).unwrap())
                    .unwrap()
                    .into()
            } else {
                DIndexVersionId::from_version_data(&file_data)
            };

            let version_id = index.insert(file_path, &file_data, parent_id).unwrap();
            let version_id_str = hex::encode(<[u8; 32]>::from(version_id));
            println!("{version_id_str}");
        }
        "get" => {
            let version_id: DIndexVersionId = if args.len() > 3 {
                TryInto::<[u8; 32]>::try_into(hex::decode(&args[3]).unwrap())
                    .unwrap()
                    .into()
            } else {
                println!("No version id provided to get operation");
                std::process::exit(1)
            };

            let file_data = index.get(file_path, version_id).unwrap().unwrap();
            println!("{file_data}");
        }
        _ => {
            println!("Invalid operation");
        }
    }
}
