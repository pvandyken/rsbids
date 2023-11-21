use std::path::PathBuf;

use pyo3::{prelude::*, types::PyDict};

use crate::layout::{bidspath::BidsPath, builders::bidspath_builder::BidsPathBuilder};

pub fn to_pybidspath(path: BidsPath) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let bidspathcls = py.import("rsbids.bidspath")?.getattr("BidsPath")?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("_entities", path.get_full_entities())?;
        kwargs.set_item("_dataset_root", path.get_root())?;

        bidspathcls
            .call((path.as_str(),), Some(kwargs))
            .map(|any| any.into())
    })
}

#[pyfunction]
pub fn create_pybidspath(path: PathBuf) -> PyResult<PyObject> {
    let builder = BidsPathBuilder::new(path, 0)?;
    match builder.spec_parse() {
        Ok(bidspath) => to_pybidspath(bidspath),
        Err(builder) => to_pybidspath(builder.get_bidspath()?),
    }
}
