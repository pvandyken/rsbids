use std::collections::{HashMap, HashSet};
use std::path::Path;

use itertools::Itertools;
use pyo3::exceptions::{PyBaseException, PyIOError, PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;

use super::pyparams::pyiterable::py_iter;
use super::{
    pybidspath::to_pybidspath,
    pyparams::{
        derivatives::{discover_derivatives, DerivativeSpecModes, DerivativesParam},
        pathlist::PathList,
        scope::ScopeList,
    },
};
use crate::layout::bidspath::BidsPath;
use crate::layout::{BidsPathViewIterator, Layout, QueryErr, QueryTerms};
use crate::fs::IterdirErr;

#[pyclass(module = "rsbids", name = "BidsPath")]
pub struct PyBidsPath {
    path: BidsPath,
}

#[pymethods]
impl PyBidsPath {
    fn __repr__(&self) -> String {
        format!("Artefact(\"{}\"", &self.path.path)
    }

    #[getter]
    fn entities(&self) -> HashMap<&str, &str> {
        self.path.get_entities()
    }
}

#[pyclass]
pub struct LayoutIterator {
    iter: BidsPathViewIterator,
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

fn map_query_terms(term: &PyAny) -> PyResult<QueryTerms> {
    if let Ok(boolean) = term.extract::<bool>() {
        Ok(QueryTerms::Bool(boolean))
    } else if let Ok(string) = term.extract::<String>() {
        Ok(QueryTerms::String(string))
    } else {
        Err(PyTypeError::new_err(
            "query terms must be strings or booleans",
        ))
    }
}

#[pyclass(module = "rsbids", name = "BidsLayout")]
pub struct PyLayout {
    raw: Layout,
}

#[pymethods]
impl PyLayout {
    #[new]
    #[pyo3(signature = (paths, derivatives=None, validate=false))]
    pub fn new(
        paths: PathList,
        derivatives: Option<DerivativesParam>,
        validate: bool,
    ) -> PyResult<Self> {
        let paths = paths.unpack()?;
        let derivatives = if let Some(d) = derivatives {
            match d.unpack()? {
                Some(DerivativeSpecModes::Set(d)) => Ok(Some(d)),
                Some(DerivativeSpecModes::Discover) => match paths.first() {
                    Some(path) => {
                        if path.len() > 1 {
                            Err(PyValueError::new_err(
                                "derivatives=True can only be specified when a single root is provided"
                            ))
                        } else {
                            Ok(discover_derivatives(Path::new(path))?)
                        }
                    }
                    None => Err(PyValueError::new_err(
                        "derivatives=True can only be specified when a root is provided",
                    )),
                },
                None => Ok(None),
            }?
        } else {
            None
        };
        Ok(PyLayout {
            raw: Layout::create(paths, derivatives, validate).map_err(|err| match err {
                IterdirErr::Interrupt(err) => err,
                IterdirErr::Io(err) => PyIOError::new_err(err),
            })?,
        })
    }

    #[getter]
    fn entities(&self) -> PyResult<HashMap<&str, Vec<&String>>> {
        Ok(self.raw.entity_fullkey_vals())
    }

    #[getter]
    fn roots(&self) -> Vec<&String> {
        self.raw.get_roots()
    }

    #[getter]
    fn root(&self) -> PyResult<&String> {
        fn try_with(r: Vec<&String>) -> PyResult<Option<&String>> {
            if r.len() > 1 {
                let mut msg = String::from("Layout is multi-root:\n");
                for root in r {
                    msg.push_str(&format!("  {}\n", root));
                }
                Err(PyValueError::new_err(msg))
            } else if let Some(root) = r.first() {
                Ok(Some(root))
            } else {
                Ok(None)
            }
        }
        if let Some(root) = try_with(self.raw.get_raw_roots())? {
            Ok(root)
        } else if let Some(root) = try_with(self.raw.get_derivative_roots())? {
            Ok(root)
        } else {
            Err(PyBaseException::new_err(
                "Unexpected problem: no roots found",
            ))
        }
    }

    #[getter]
    fn derivatives(&self) -> PyResult<Self> {
        let deriv_roots = self
            .raw
            .roots
            .derivative_keys()
            .map(|s| s.to_string())
            .collect_vec();
        if deriv_roots.len() == 0 {
            return Err(PyValueError::new_err("Layout has no derivatives"));
        }
        Ok(Self {
            raw: self
                .raw
                .query(None, Some(deriv_roots))
                .map_err(|err| PyBaseException::new_err(format!("Unexpected error: {}", err)))?,
        })
    }

    fn debug_roots(&self) -> String {
        self.raw.display_root_ranges()
    }

    #[pyo3(signature = (root=None, scope=None, **entities))]
    fn get(
        &self,
        root: Option<PathList>,
        scope: Option<ScopeList>,
        entities: Option<HashMap<String, Py<PyAny>>>,
    ) -> PyResult<PyLayout> {
        Python::with_gil(|py| {
            // Normalize entities
            let entities = entities
                .map(|entities| {
                    entities
                        .into_iter()
                        .filter_map(|(entity, query)| {
                            if query.is_none(py) {
                                return None;
                            }
                            if let Ok(s) = map_query_terms(query.as_ref(py)) {
                                Some(Ok((entity, vec![s])))
                            } else {
                                Some(match py_iter(query.as_ref(py)) {
                                    Ok(iterator) => {
                                        match iterator
                                            .as_ref(py)
                                            .map(|obj| map_query_terms(obj?))
                                            .collect::<PyResult<Vec<_>>>()
                                        {
                                            Ok(terms) => Ok((entity, terms)),
                                            Err(err) => Err(err),
                                        }
                                    }
                                    Err(_) => match map_query_terms(query.as_ref(py)) {
                                        Ok(term) => Ok((entity, vec![term])),
                                        Err(err) => Err(err),
                                    },
                                })
                            }
                        })
                        .collect::<PyResult<HashMap<String, Vec<QueryTerms>>>>()
                })
                .transpose()?;

            // Normalize scope
            let scopes = scope
                .map(|scope| -> PyResult<_> {
                    self.raw
                        .get_scopes(scope.unpack()?)
                        .map_err(|err| PyValueError::new_err(format!("{}", err)))
                })
                .transpose()?
                .flatten()
                .map(|scope| scope);

            // Normalize Root
            let mut root = root.map(|root| root.unpack()).transpose()?;
            if let Some(scopes) = scopes {
                if let Some(root) = &mut root {
                    root.extend(scopes)
                } else {
                    root = Some(scopes)
                }
            }

            match self.raw.query(entities, root) {
                Ok(raw) => Ok(PyLayout { raw }),
                Err(err) => Err(match err {
                    QueryErr::MissingVal(..) | QueryErr::GlobErr(..) => PyValueError::new_err,
                    QueryErr::MissingEntity(..) => PyKeyError::new_err,
                }(format!("{}", err))),
            }
        })
    }

    #[pyo3(signature = (root=None, scope=None, **entities))]
    fn getone(
        &self,
        root: Option<PathList>,
        scope: Option<ScopeList>,
        entities: Option<HashMap<String, Py<PyAny>>>,
    ) -> PyResult<PyObject> {
        let results = self.get(root, scope, entities)?;
        if results.raw.len() == 0 {
            Err(PyValueError::new_err("No items returned from query"))
        } else if results.raw.len() > 1 {
            let mut msg = String::from("Expected one path, but got:\n");
            msg.push_str(&results.raw.fmt_elided_list(5));
            let problem_entities: HashMap<_, _> = results
                .raw
                .entity_key_vals()
                .into_iter()
                .filter_map(|(key, val)| {
                    if val.len() > 1 {
                        Some((key, val))
                    } else {
                        None
                    }
                })
                .collect();
            msg.push_str("\n\nThe following entities remain to be filtered:\n");
            msg.push_str(&format!("{:#?}", problem_entities));
            Err(PyValueError::new_err(msg))
        } else {
            Ok(to_pybidspath(results.raw.get_path(0).unwrap())?)
        }
    }

    fn __getitem__(&self, i: usize) -> PyResult<PyObject> {
        match self.raw.get_path(i).map(|path| to_pybidspath(path)) {
            Some(path) => path,
            None => Err(PyKeyError::new_err(format!("Index {} out of range", i))),
        }
    }

    fn __len__(&self) -> usize {
        self.raw.len()
    }

    fn __repr__(&self) -> String {
        let mut repr = String::from(format!("<BidsLayout (len = {})>\n", self.raw.len()));
        let interesting_entities = HashSet::from(["subject", "session", "run"]);
        let entities = self.raw.entity_fullkey_vals();
        let kept_entities = entities
            .iter()
            .filter_map(|(key, val)| {
                if interesting_entities.contains(key as &str) {
                    Some((key, val.len()))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();
        repr.push_str("Entities:\n");
        for (key, val) in &kept_entities {
            repr.push_str(&format!("    {}: {}\n", key, val));
        }
        repr.push_str(&format!(
            "Other entities: {}\n",
            entities
                .keys()
                .filter(|key| { !kept_entities.contains_key(key) })
                .join(", ")
        ));
        repr.push_str(&self.raw.fmt_elided_list(10));
        repr
    }

    fn __iter__(&self) -> LayoutIterator {
        LayoutIterator {
            iter: self.raw.get_paths(),
        }
    }
}
