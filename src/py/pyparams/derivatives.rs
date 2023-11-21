use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use itertools::Itertools;
use pyo3::{types::PyIterator, FromPyObject, PyErr, PyResult};

use crate::pyiterable;

#[derive(FromPyObject, Debug)]
pub enum DerivParamPrimitive {
    String(String),
    Path(PathBuf),
}

impl From<DerivParamPrimitive> for PathBuf {
    fn from(value: DerivParamPrimitive) -> Self {
        match value {
            DerivParamPrimitive::Path(path) => path,
            DerivParamPrimitive::String(string) => PathBuf::from(string),
        }
    }
}

pyiterable!(DerivParamCollection<DerivParamPrimitive>);

#[derive(FromPyObject)]
pub enum DerivativesParam<'a> {
    Bool(bool),
    Primitive(DerivParamPrimitive),
    Map(HashMap<String, DerivParamCollection<'a>>),
    Single(DerivParamCollection<'a>),
}

impl DerivativesParam<'_> {
    pub fn unpack(self) -> PyResult<Option<DerivativeSpecModes>> {
        let params: Vec<DerivativeSpec> = match self {
            Self::Single(param) => param.try_into()?,
            Self::Map(map) => {
                let mut result: Vec<DerivativeSpec> = Vec::new();
                for (label, params) in map {
                    let mut spec: DerivativeSpec = params.try_into()?;
                    spec.label = Some(label);
                    result.push(spec)
                }
                result
            }
            Self::Primitive(param) => vec![param.into()],
            Self::Bool(bool) => match bool {
                false => return Ok(None),
                true => return Ok(Some(DerivativeSpecModes::Discover)),
            },
        };
        Ok(Some(DerivativeSpecModes::Set(params)))
    }
}

#[derive(Debug)]
pub struct DerivativeSpec {
    pub label: Option<String>,
    pub paths: Vec<PathBuf>,
}

impl From<PathBuf> for DerivativeSpec {
    fn from(value: PathBuf) -> Self {
        Self {
            paths: vec![value],
            label: None,
        }
    }
}

impl From<String> for DerivativeSpec {
    fn from(value: String) -> Self {
        Self {
            label: None,
            paths: vec![PathBuf::from(value)],
        }
    }
}

impl From<DerivParamPrimitive> for DerivativeSpec {
    fn from(value: DerivParamPrimitive) -> Self {
        Self {
            label: None,
            paths: vec![value.into()],
        }
    }
}

impl<'a> TryFrom<DerivParamCollection<'a>> for DerivativeSpec {
    type Error = PyErr;
    fn try_from(value: DerivParamCollection<'a>) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl TryFrom<&PyIterator> for DerivativeSpec {
    type Error = PyErr;
    fn try_from(value: &PyIterator) -> PyResult<Self> {
        Ok(Self {
            label: None,
            paths: value
                .map(|x| Ok(x?.extract::<DerivParamPrimitive>()?.into()))
                .collect::<PyResult<_>>()?,
        })
    }
}

impl FromIterator<DerivParamPrimitive> for DerivativeSpec {
    fn from_iter<T: IntoIterator<Item = DerivParamPrimitive>>(iter: T) -> Self {
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
