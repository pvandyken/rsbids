use pyo3::prelude::*;

use crate::layout::BidsPathViewIterator;

use super::pybidspath::to_pybidspath;

#[pyclass(module = "rsbids", name = "BidsLayoutIterator")]
pub struct LayoutIterator {
    pub iter: BidsPathViewIterator,
}

#[pymethods]
impl LayoutIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PyObject>> {
        slf.iter.next().map(|obj| to_pybidspath(obj)).transpose()
    }
}