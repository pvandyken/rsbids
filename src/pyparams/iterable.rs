use itertools::Itertools;
use pyo3::{types::PyIterator, FromPyObject, PyErr, PyResult};


#[derive(FromPyObject)]
pub enum IterableParamTypes<'a, I>
where
    I: FromPyObject<'a>,
{
    Primitive(I),
    Vec(Vec<I>),
    Iter(&'a PyIterator),
}

#[derive(FromPyObject)]
#[pyo3(transparent)]
pub struct IterableParam<'a, I>
where
    I: FromPyObject<'a>,
{
    data: IterableParamTypes<'a, I>,
}

impl<'a, I, J> TryInto<Vec<J>> for IterableParam<'a, I>
where
    I: FromPyObject<'a> + Into<J>,
    J: FromPyObject<'a>,
{
    type Error = PyErr;
    fn try_into(self) -> Result<Vec<J>, Self::Error> {
        Ok(match self.data {
            IterableParamTypes::Primitive(prim) => vec![prim.into()],
            IterableParamTypes::Vec(paths) => paths.into_iter().map_into().collect(),
            IterableParamTypes::Iter(iter) => iter
                .map(|o| Ok(o?.extract()?))
                .collect::<PyResult<Vec<_>>>()?,
        })
    }
}
