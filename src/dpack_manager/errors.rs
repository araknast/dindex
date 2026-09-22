use thiserror::Error;

#[derive(Debug, Error)]
pub enum GetHeadError {
    #[error("I/O error reading head")]
    Io(#[from] std::io::Error),
    #[error("Could not load head DPack")]
    DPackLoad(#[from] DPackLoadError),
}

#[derive(Debug, Error)]
pub enum DPackPersistError {
    #[error("I/O Error persisting DPack")]
    Io(#[from] std::io::Error),
    #[error("Could not persist DPack: could not load index")]
    IndexParse(#[from] DPackIndexLoadError),
    #[error("Could not persist DPack: could not load DPack")]
    DPackLoad(#[from] DPackLoadError),
    #[error("Could not persist DPack: could not get head")]
    GetHead(#[from] GetHeadError),
}

#[derive(Debug, Error)]
pub enum DPackLoadError {
    #[error("I/O Error loading DPack")]
    Io(#[from] std::io::Error),
    #[error("Could not load DPack: could not load index")]
    IndexLoad(#[from] DPackIndexLoadError),
}

#[derive(Debug, Error)]
pub enum DPackIndexLoadError {
    #[error("File ended early.")]
    EarlyTermination,
    #[error("Could not read index file")]
    FileLoad(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("I/O Error initializing DPack manager")]
    Io(#[from] std::io::Error),
}
