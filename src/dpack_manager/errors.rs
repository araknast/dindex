use thiserror::Error;

#[derive(Debug, Error)]
pub enum DPackPersistError {
    #[error("I/O Error persisting DPack")]
    Io(#[from] std::io::Error),
    #[error("Could not persist DPack: could not load index")]
    IndexParse(#[from] DPackIndexLoadError),
}

#[derive(Debug, Error)]
pub enum DPackLoadError {
    #[error("I/O Error loading DPack")]
    Io(#[from] std::io::Error),
    #[error("Could not load DPack: could not load index")]
    IndexParse(#[from] DPackIndexLoadError),
}

#[derive(Debug, Error)]
pub enum DPackIndexLoadError {
    #[error("File ended early.")]
    EarlyTermination,
    #[error("Could not read index file")]
    FileLoad(#[from] std::io::Error),
}
