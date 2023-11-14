use crate::layout::Layout;
use pybidspath::create_pybidspath;
use pyo3::prelude::*;

pub mod bidspath;
pub mod dataset;
pub mod fs;
pub mod layout;
pub mod primitives;
pub mod standards;
pub mod utils;
pub mod dataset_description;
pub mod pyparams;
pub mod pybidspath;

/// A Python module implemented in Rust.
#[pymodule]
fn rsbids(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Layout>()?;
    m.add_function(wrap_pyfunction!(create_pybidspath, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::dataset::Dataset;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4)
    }

    #[test]
    fn main() {
        let _ = Dataset::create(vec!["data".to_string()], None);
    }
}
