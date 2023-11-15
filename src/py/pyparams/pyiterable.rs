use pyo3::{types::PyIterator, FromPyObject, Py, PyAny, PyResult, Python};

pub fn py_iter(pyobj: &PyAny) -> PyResult<Py<PyIterator>> {
    Python::with_gil(|py| {
        let builtins = py.import("builtins")?;
        let iter = builtins.getattr("iter")?;
        match iter.call1((pyobj,)) {
            Ok(iterator) => match iterator.downcast::<PyIterator>() {
                Ok(iter) => Ok(iter.into()),
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err),
        }
    })
}


pub struct PyIterable<I> {
    pub data: Vec<I>,
}


impl<'a, I: FromPyObject<'a>> PyIterable<I> {
    pub fn collect(py: Python<'a>, ob: &'a PyAny) -> PyResult<Vec<I>> {
        let builtins = py.import("builtins")?;
        let iter = builtins.getattr("iter")?;
        match iter.call1((ob,)) {
            Ok(iterator) => match iterator.downcast::<PyIterator>() {
                Ok(iter) => {
                    iter.map(|o| o?.extract()).collect()
                },
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err),
        }
    }

}