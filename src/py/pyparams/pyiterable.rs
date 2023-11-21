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

#[macro_export]
macro_rules! pyiterable {
    ($enum_name:ident<$inner_ty:ty>) => {
        #[derive(pyo3::FromPyObject)]
        pub enum $enum_name<'a> {
            Primitive($inner_ty),
            Vec(Vec<$inner_ty>),
            Iter(&'a pyo3::types::PyIterator),
            Iterable(crate::py::pyparams::pyiterable::PyIterable<$inner_ty>),
        }

        impl<'a> pyo3::FromPyObject<'a> for crate::py::pyparams::pyiterable::PyIterable<$inner_ty> {
            fn extract(ob: &'a pyo3::PyAny) -> pyo3::PyResult<Self> {
                Ok(Self {
                    data: pyo3::Python::with_gil(|py| Self::collect(py, ob))?,
                })
            }
        }

        impl<'a, J> TryFrom<$enum_name<'a>> for Vec<J>
        where
            J: From<$inner_ty>,
        {
            type Error = pyo3::PyErr;
            fn try_from(value: $enum_name<'a>) -> Result<Vec<J>, Self::Error> {
                Ok(match value {
                    $enum_name::Primitive(prim) => vec![prim.into()],
                    $enum_name::Vec(paths) => paths.into_iter().map(|x| x.into()).collect(),
                    $enum_name::Iter(iter) => iter
                        .map(|o| Ok(o?.extract::<$inner_ty>()?.into()))
                        .collect::<pyo3::PyResult<Vec<_>>>()?,
                    $enum_name::Iterable(iter) => iter.data.into_iter().map(|x| x.into()).collect(),
                })
            }
        }

    };
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