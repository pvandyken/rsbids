use itertools::Itertools;
use pyo3::{FromPyObject, PyAny, PyResult, Python};

use crate::utils::PyIterable;

use super::iterable::IterableParam;

#[derive(FromPyObject)]
pub enum ScopeListType<'a> {
    Regular(IterableParam<'a, String>),
    Irregular(PyIterable<String>),
}

impl<'a> FromPyObject<'a> for PyIterable<String> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        Ok(Self {
            data: Python::with_gil(|py| Self::collect(py, ob))?,
        })
    }
}

#[derive(FromPyObject)]
#[pyo3(transparent)]
pub struct ScopeList<'a> {
    param: ScopeListType<'a>,
}

impl ScopeList<'_> {
    pub fn unpack(self) -> PyResult<Vec<String>> {
        Ok(match self.param {
            ScopeListType::Regular(prim) => prim.try_into()?,
            ScopeListType::Irregular(paths) => paths.data.into_iter().map_into().collect(),
        })
    }
}
