use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use itertools::Itertools;
use pyo3::{types::PyIterator, FromPyObject, PyAny, PyErr, PyResult, Python};


use super::{iterable::IterableParam, pyiterable::PyIterable};

#[derive(FromPyObject)]
pub enum DerivativesParamPrimitiveType {
    String(String),
    Path(PathBuf),
}

impl Into<String> for DerivativesParamPrimitiveType {
    fn into(self) -> String {
        match self {
            Self::Path(path) => path.to_string_lossy().to_string(),
            Self::String(string) => string,
        }
    }
}

#[derive(FromPyObject)]
enum DerivativesParamCollectionType<'a> {
    Regular(IterableParam<'a, DerivativesParamPrimitiveType>),
    Iterable(PyIterable<DerivativesParamPrimitiveType>),
}

impl<'a> FromPyObject<'a> for PyIterable<DerivativesParamPrimitiveType> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        Ok(Self {
            data: Python::with_gil(|py| Self::collect(py, ob))?,
        })
    }
}

#[derive(FromPyObject)]
enum DerivativesParamType<'a> {
    Bool(bool),
    Primitive(DerivativesParamPrimitiveType),
    Map(HashMap<String, DerivativesParamCollectionType<'a>>),
    Single(DerivativesParamCollectionType<'a>),
}

#[derive(FromPyObject)]
#[pyo3(transparent)]
pub struct DerivativesParam<'a> {
    param: DerivativesParamType<'a>,
}

impl DerivativesParam<'_> {
    pub fn unpack(self) -> PyResult<Option<DerivativeSpecModes>> {
        let mut result: Vec<DerivativeSpec> = Vec::new();
        match self.param {
            DerivativesParamType::Single(param) => result.push(param.try_into()?),
            DerivativesParamType::Map(map) => {
                for (label, params) in map {
                    let mut spec: DerivativeSpec = params.try_into()?;
                    spec.label = Some(label);
                    result.push(spec)
                }
            }
            DerivativesParamType::Primitive(param) => result.push(param.into()),
            DerivativesParamType::Bool(bool) => match bool {
                false => return Ok(None),
                true => return Ok(Some(DerivativeSpecModes::Discover)),
            },
        };
        Ok(Some(DerivativeSpecModes::Set(result)))
    }
}

pub struct DerivativeSpec {
    pub label: Option<String>,
    pub paths: Vec<String>,
}

impl From<PathBuf> for DerivativeSpec {
    fn from(value: PathBuf) -> Self {
        Self {
            paths: vec![value.to_string_lossy().to_string()],
            label: None,
        }
    }
}

impl From<String> for DerivativeSpec {
    fn from(value: String) -> Self {
        Self {
            label: None,
            paths: vec![value],
        }
    }
}

impl From<DerivativesParamPrimitiveType> for DerivativeSpec {
    fn from(value: DerivativesParamPrimitiveType) -> Self {
        Self {
            label: None,
            paths: vec![value.into()],
        }
    }
}

impl TryFrom<DerivativesParamCollectionType<'_>> for DerivativeSpec {
    type Error = PyErr;
    fn try_from(value: DerivativesParamCollectionType) -> Result<Self, Self::Error> {
        Ok(match value {
            DerivativesParamCollectionType::Regular(data) => {
                DerivativeSpec::from_vec(data.try_into()?)
            }
            DerivativesParamCollectionType::Iterable(iter) => iter.data.into_iter().collect(),
        })
    }
}
impl DerivativeSpec {
    fn from_vec(value: Vec<String>) -> Self {
        Self {
            label: None,
            paths: value,
        }
    }
}

impl TryFrom<&PyIterator> for DerivativeSpec {
    type Error = PyErr;
    fn try_from(value: &PyIterator) -> PyResult<Self> {
        Ok(Self {
            label: None,
            paths: value
                .map(|x| Ok(x?.extract::<DerivativesParamPrimitiveType>()?.into()))
                .collect::<PyResult<_>>()?,
        })
    }
}

impl FromIterator<DerivativesParamPrimitiveType> for DerivativeSpec {
    fn from_iter<T: IntoIterator<Item = DerivativesParamPrimitiveType>>(iter: T) -> Self {
        Self {
            label: None,
            paths: iter.into_iter().map_into().collect(),
        }
    }
}

pub enum DerivativeSpecModes {
    Set(Vec<DerivativeSpec>),
    Discover,
}

pub fn discover_derivatives(root: &Path) -> io::Result<Option<Vec<DerivativeSpec>>> {
    let deriv = root.join("derivatives");
    if deriv.is_dir() {
        let mut result = Vec::new();
        for path in fs::read_dir(deriv)? {
            let path = path?.path();
            if path.is_dir() {
                if let Some(mut sub_derivs) = discover_derivatives(&path)? {
                    result.extend(sub_derivs.drain(..));
                }
                result.push(path.into())
            }
        }
        Ok(Some(result))
    } else {
        Ok(None)
    }
}
