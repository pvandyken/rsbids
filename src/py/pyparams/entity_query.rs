use std::collections::HashMap;

use pyo3::{FromPyObject, PyResult};

use crate::{layout::QueryTerms, pyiterable};

#[derive(pyo3::FromPyObject)]
pub enum QueryPrimitives {
    String(String),
    Bool(bool),
    Number(u64),
}

impl From<Option<QueryPrimitives>> for QueryTerms {
    fn from(value: Option<QueryPrimitives>) -> Self {
        match value {
            Some(QueryPrimitives::Bool(b)) => Self::Bool(b),
            Some(QueryPrimitives::String(s)) => Self::String(s),
            Some(QueryPrimitives::Number(x)) => Self::Number(x),
            None => Self::Any,
        }
    }
}

pyiterable!(QueryParamsType<Option<QueryPrimitives>>);

#[derive(FromPyObject)]
#[pyo3(transparent)]
pub struct QueryParams<'a>(HashMap<String, Option<QueryParamsType<'a>>>);

impl QueryParams<'_> {
    pub fn unpack(self) -> PyResult<HashMap<String, Vec<QueryTerms>>> {
        let mut result: HashMap<String, Vec<QueryTerms>> = HashMap::new();
        for (key, val) in self.0 {
            if let Some(val) = val {
                result.insert(key, val.try_into()?);
            }
        }
        Ok(result)
    }
}
