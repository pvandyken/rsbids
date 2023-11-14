use std::path::PathBuf;

use pyo3::{prelude::*, types::PyDict};

use crate::bidspath::{BidsPath, BidsPathBuilder};

pub fn to_pybidspath(path: BidsPath) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let bidspathcls = py.import("rsbids.bidspath")?.getattr("BidsPath")?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("entities", path.get_entities())?;
        kwargs.set_item("dataset_root", path.get_root())?;

        bidspathcls
            .call((path.path,), Some(kwargs))
            .map(|any| any.into())
    })
}

#[pyfunction]
pub fn create_pybidspath(path: PathBuf) -> PyResult<PyObject> {
    let builder = BidsPathBuilder::new(path.to_string_lossy().to_string(), 0);
    match builder.via_spec() {
        Ok(bidspath) => to_pybidspath(bidspath),
        Err(builder) => to_pybidspath(BidsPath::new(builder.path, builder.root)),
    }
}
