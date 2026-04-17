pub mod config;
pub mod export;
pub mod fs;
pub mod markdown;
pub mod update;
pub mod window_mgmt;

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Fs(#[from] crate::fs_ops::FsError),
    #[error("{0}")]
    Generic(String),
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
