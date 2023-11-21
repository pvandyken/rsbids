use pyo3::{
    exceptions::{PyIOError, PyKeyError, PyUnicodeError, PyValueError},
    prelude::*,
};

pub use pyo3::PyResult;

use crate::errors::{BidsPathErr, CacheErr, IterdirErr, QueryErr};

impl From<BidsPathErr> for PyErr {
    fn from(value: BidsPathErr) -> PyErr {
        match value {
            BidsPathErr::Encoding(..) => PyUnicodeError::new_err(format!("{}", value)),
            BidsPathErr::Validation(..) => PyValueError::new_err(format!("{}", value)),
        }
    }
}

impl From<IterdirErr> for PyErr {
    fn from(value: IterdirErr) -> Self {
        match value {
            IterdirErr::Interrupt(err) => err,
            IterdirErr::Io(err) => PyIOError::new_err(err),
        }
    }
}

impl From<QueryErr> for PyErr {
    fn from(value: QueryErr) -> Self {
        match value {
            QueryErr::MissingVal(..)
            | QueryErr::GlobErr(..)
            | QueryErr::MutliErr(..)
            | QueryErr::AmbiguousQuery(..) => PyValueError::new_err(format!("{}", value)),
            QueryErr::MissingEntity(..) => PyKeyError::new_err(format!("{}", value)),
        }
    }
}

impl From<CacheErr> for PyErr {
    fn from(value: CacheErr) -> Self {
        PyIOError::new_err(format!("{}", value))
    }
}
