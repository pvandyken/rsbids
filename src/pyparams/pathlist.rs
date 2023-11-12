use std::path::PathBuf;

use itertools::Itertools;
use pyo3::{FromPyObject, PyAny, PyResult, Python};

use crate::utils::PyIterable;

use super::iterable::IterableParam;

#[derive(FromPyObject)]
pub enum LayoutRootPrimitive {
    String(String),
    Path(PathBuf),
}

impl Into<String> for LayoutRootPrimitive {
    fn into(self) -> String {
        match self {
            LayoutRootPrimitive::Path(path) => path.to_string_lossy().to_string(),
            LayoutRootPrimitive::String(string) => string,
        }
    }
}

#[derive(FromPyObject)]
pub enum GenericPathParamTypes<'a> {
    Regular(IterableParam<'a, LayoutRootPrimitive>),
    Irregular(PyIterable<LayoutRootPrimitive>),
}

impl<'a> FromPyObject<'a> for PyIterable<LayoutRootPrimitive> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        Ok(Self {
            data: Python::with_gil(|py| Self::collect(py, ob))?,
        })
    }
}

#[derive(FromPyObject)]
#[pyo3(transparent)]
pub struct PathList<'a> {
    param: GenericPathParamTypes<'a>,
}

impl PathList<'_> {
    pub fn unpack(self) -> PyResult<Vec<String>> {
        Ok(match self.param {
            GenericPathParamTypes::Regular(prim) => prim.try_into()?,
            GenericPathParamTypes::Irregular(paths) => paths.data.into_iter().map_into().collect(),
        })
    }
}
