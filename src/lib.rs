use pyo3::prelude::*;
use crate::layout::Layout;

pub mod bidspath;
pub mod dataset;
pub mod primitives;
pub mod standards;
pub mod layout;
pub mod fs;



/// A Python module implemented in Rust.
#[pymodule]
fn rsbids(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Layout>()?;
    Ok(())
}
