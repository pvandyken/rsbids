use std::path::PathBuf;

use pyo3::{FromPyObject,  PyResult, PyErr};

use crate::pyiterable;


#[derive(FromPyObject)]
pub enum LayoutRootPrimitive {
    String(String),
    Path(PathBuf),
}

impl From<LayoutRootPrimitive> for PathBuf {
    fn from(value: LayoutRootPrimitive) -> Self {
        match value {
            LayoutRootPrimitive::Path(path) => path,
            LayoutRootPrimitive::String(string) => PathBuf::from(string),
        }
    }
}

pyiterable!(GenericPathParamTypes<LayoutRootPrimitive>);

#[derive(FromPyObject)]
#[pyo3(transparent)]
pub struct PathList<'a> {
    param: GenericPathParamTypes<'a>,
}

impl PathList<'_> {
    pub fn unpack(self) -> PyResult<Vec<PathBuf>> {
        self.try_into()
    }
}

impl<'a> TryFrom<PathList<'a>> for Vec<PathBuf> {
    type Error = PyErr;
    fn try_from(value: PathList<'a>) -> Result<Self, Self::Error> {
        value.param.try_into()
        
    }
}
