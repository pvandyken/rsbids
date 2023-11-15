use crate::py::pybidspath::create_pybidspath;
use crate::py::pylayout::PyLayout;
use pyo3::prelude::*;

pub mod layout;
pub mod dataset_description;
pub mod fs;
pub mod py;
pub mod standards;

/// A Python module implemented in Rust.
#[pymodule]
fn rsbids(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyLayout>()?;
    m.add_function(wrap_pyfunction!(create_pybidspath, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::layout::Layout;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4)
    }

    #[test]
    fn main() {
        let _ = Layout::create(vec!["data".to_string()], None, false);
    }
}
