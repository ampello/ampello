// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Invalid(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    Internal(String),
}

impl Error {
    pub fn not_found(what: impl Into<String>) -> Self {
        Error::NotFound(what.into())
    }
    pub fn invalid(what: impl Into<String>) -> Self {
        Error::Invalid(what.into())
    }
    pub fn conflict(what: impl Into<String>) -> Self {
        Error::Conflict(what.into())
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
