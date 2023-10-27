use std::path::PathBuf;

use itertools::Itertools;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::dataset::Dataset;
use crate::fs::iterdir;

#[pyclass]
pub struct Layout {
    raw: Dataset,
}

#[pymethods]
impl Layout {
    #[new]
    fn new(paths: String) -> PyResult<Self> {
        let mut dataset = Dataset::default();
        match PathBuf::from(paths).canonicalize() {
            Ok(elem) => match iterdir(elem, |path| dataset.add_path(path)) {
                Ok(..) => Ok(()),
                Err(e) => Err(PyIOError::new_err(e.to_string())),
            },
            Err(e) => Err(PyIOError::new_err(e.to_string())),
        }
        .map(|_| {
            dataset.cleanup();
            Layout { raw: dataset }
        })
    }

    fn __str__(&self) -> String {
        let sample = (0..5)
            .map(|i| self.raw.get_path(i))
            .filter(|p| p.is_some())
            .map(|p| p.unwrap())
            .collect_vec();
        let count = sample.len();
        return format!(
            "The dataset has {} paths. The first {} are {}",
            self.raw.num_paths(),
            count,
            sample.join(", ")
        );
    }
}
