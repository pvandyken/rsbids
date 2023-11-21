use crate::py::pybidspath::create_pybidspath;
use crate::py::pylayout::PyLayout;
use py::pydescription::{PyDatasetDescription, PyGeneratedBy, PySourceDataset};
use py::pylayout_iterator::LayoutIterator;
use standards::deref_key_alias;
use crate::standards::get_key_alias;
use pyo3::prelude::*;

pub mod layout;
pub mod dataset_description;
pub mod fs;
pub mod py;
pub mod utils;
pub mod standards;
pub mod serialize;
pub mod errors;


#[pyfunction]
fn entity_long_to_short(e: &str) -> &str {
    deref_key_alias(e).unwrap_or(e)
}

#[pyfunction]
fn entity_short_to_long(e: &str) -> &str {
    get_key_alias(e)
}

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_lib")]
fn rsbids(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyLayout>()?;
    m.add_class::<LayoutIterator>()?;
    m.add_class::<PyDatasetDescription>()?;
    m.add_class::<PyGeneratedBy>()?;
    m.add_class::<PySourceDataset>()?;
    m.add_function(wrap_pyfunction!(create_pybidspath, m)?)?;
    m.add_function(wrap_pyfunction!(entity_long_to_short, m)?)?;
    m.add_function(wrap_pyfunction!(entity_short_to_long, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::layout::Layout;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4)
    }

    #[test]
    fn main() {
        let _ = Layout::create(vec![PathBuf::from("data")], None, false);
    }
}
