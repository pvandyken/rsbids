use itertools::Itertools;
use std::{io, path::PathBuf};

use pyo3::PyErr;
use thiserror::Error;

use crate::layout::bidspath::BidsPath;

#[derive(Debug)]
pub enum DatasetDescriptionErr {
    IoErr(io::Error),
    JsonErr(serde_json::Error),
}

#[derive(Error, Debug)]
pub enum IterdirErr {
    #[error("{0}")]
    Io(io::Error),
    #[error("{0}")]
    Interrupt(PyErr),
}

#[derive(Error, Debug)]
pub enum BidsPathErr {
    #[error("'{0}' is not valid unicode")]
    Encoding(PathBuf),
    #[error("'{}' is not a valid bids path", .0.as_str())]
    Validation(BidsPath),
}

impl From<PathBuf> for BidsPathErr {
    fn from(value: PathBuf) -> Self {
        Self::Encoding(value)
    }
}

impl BidsPathErr {
    pub fn get_bidspath(self) -> Result<BidsPath, Self> {
        match self {
            Self::Encoding(..) => Err(self),
            Self::Validation(p) => Ok(p),
        }
    }
}

#[derive(Error, Debug)]
pub enum GlobErr {
    #[error(transparent)]
    Glob(#[from] globset::Error),
    #[error("'{0}' is not valid unicode")]
    Encoding(PathBuf),
}

#[derive(Error, Debug)]
pub enum QueryErr {
    #[error("Entity not found {0:?}")]
    MissingEntity(Vec<String>),
    #[error("Could not find values: {1:?} for entity: '{0}'")]
    MissingVal(String, Vec<String>),
    #[error("Query '{0}={1}' matched multiple possible values: {2:?}. Please use a string query to be more specific")]
    AmbiguousQuery(String, u64, Vec<String>),
    #[error("Multiple Query errors:\n{}", .0.iter().map(|err| format!("{}", err)).join("\n"))]
    MutliErr(Vec<QueryErr>),
    #[error(transparent)]
    GlobErr(#[from] GlobErr),
}

#[derive(Error, Debug)]
pub enum MetadataReadErr {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Error parsing {0}: {1}")]
    Json(String, serde_json::Error),
    #[error("Error parsing {0}: Json must have an object as root")]
    Format(String),
}

#[derive(Error, Debug)]
pub enum MetadataIndexErr {
    #[error(transparent)]
    Read(#[from] MetadataReadErr),
    #[error(transparent)]
    Query(#[from] QueryErr),
}

#[derive(Error, Debug)]
pub enum CacheErr {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] bincode::Error),
}
