use std::io;

use thiserror::Error;

use crate::{
    blob_manager, dindex,
    dpack_manager::{self, DPackLoadError, DPackPersistError},
};

#[derive(Debug, Error)]
#[error("Failed to parse snapshot data")]
pub enum SnapshotReadError {
    #[error("Could not parse snapshot id")]
    ObjectIdParse(#[from] hex::FromHexError),
    #[error("File ended early")]
    EarlyTermination,
    #[error("Invalid snapshot id")]
    InvalidId(#[from] dindex::DeserializationError),
}

#[derive(Debug, Error)]
pub enum SnapshotPersistError {
    #[error("Could not insert into DIndex: DIndex does not exist")]
    Nonexistent,
    #[error("Could not insert into DIndex: could not load DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not insert into DIndex: could not persist DPack")]
    DPackPersist(#[from] DPackPersistError),
}

#[derive(Debug, Error)]
pub enum SnapshotLoadError {
    #[error("Error reading snapshot data")]
    Read(#[from] SnapshotReadError),
    #[error("Could not load snapshot: could not load the snapshot index")]
    IndexLoad(#[from] SnapshotIndexLoadError),
    #[error("Could not load snapshot: snapshot does not exist")]
    Nonexistent,
}

#[derive(Debug, Error)]
pub enum SnapshotReproductionError {
    #[error("Could not reproduce snapshot: could not load snapshot")]
    SnapshotLoad(#[from] SnapshotLoadError),
    #[error("Could not reproduce snapshot: could not load a DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not reproduce snapshot: I/O error")]
    IO(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum SnapshotIndexLoadError {
    #[error("Could not load the snapshot DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Snapshot index does not exist in snapshot DPack!")]
    NoIndex,
}

#[derive(Debug, Error)]
pub enum SnapshotCreationError {
    #[error("I/O error attempting to create snapshot")]
    IO(#[from] io::Error),
    #[error("Could not create snapshot: could not update a file's Dindex")]
    DIndexInsert(#[from] DIndexInsertError),
    #[error("Could not create snapshot: could not persist snapshot")]
    SnapshotPersist(#[from] SnapshotPersistError),
    #[error("Could not create snapshot: could not get head")]
    GetHead(#[from] GetHeadError),
    #[error("Could not create snapshot: invalid target directory")]
    InvalidTarget,
}

#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("Could not initialize dpack manager")]
    DPackManager(#[from] dpack_manager::InitializationError),
    #[error("Could not initialize blob manager")]
    BlobManager(#[from] blob_manager::InitializationError),
}

#[derive(Debug, Error)]
pub enum GetHeadError {
    #[error("Could not get head: could not load snap index")]
    IndexLoad(#[from] SnapshotIndexLoadError),
}

#[derive(Debug, Error)]
pub enum DIndexInsertError {
    #[error("Could not insert into DIndex: DIndex does not exist")]
    Nonexistent,
    #[error("Could not insert into DIndex: could not load DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not insert into DIndex: could not persist DPack")]
    DPackPersist(#[from] DPackPersistError),
}
