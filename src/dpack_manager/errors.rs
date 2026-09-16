use std::io;

use crate::dindex::DIndex;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to persist DPack: {source}")]
pub struct DPackPersistError {
    pub index: DIndex,
    pub source: io::Error,
}

#[derive(Debug, Error)]
pub enum DPackIndexParseError {
    #[error("File ended early.")]
    EarlyTermination,
    #[error("Could not read index file")]
    FileLoad(#[from] std::io::Error),
}
