use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use itertools::Itertools;
use pyo3::exceptions::{PyAttributeError, PyBaseException, PyException, PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyType;

use super::pydescription::PyDatasetDescription;
use super::pylayout_iterator::LayoutIterator;
use super::pyparams::derivatives::DerivativeSpec;
use super::pyparams::entity_query::QueryParams;
use super::{
    pybidspath::to_pybidspath,
    pyparams::{
        derivatives::{discover_derivatives, DerivativeSpecModes, DerivativesParam},
        pathlist::PathList,
        scope::ScopeList,
    },
};
use crate::dataset_description::DatasetDescription;
use crate::layout::cache::LayoutCache;
use crate::layout::roots::RootCategory;
use crate::layout::Layout;

#[pyclass(module = "rsbids", name = "BidsLayout")]
pub struct PyLayout {
    pub inner: Layout,
}

#[pymethods]
impl PyLayout {
    #[new]
    #[pyo3(signature = (roots=None, derivatives=None, validate=false, cache=None, reset_cache=false))]
    pub fn new(
        roots: Option<PathList>,
        derivatives: Option<DerivativesParam>,
        validate: bool,
        cache: Option<PathBuf>,
        reset_cache: bool,
    ) -> PyResult<Self> {
        let paths = roots
            .map(|r| Ok::<_, PyErr>(r.unpack()?))
            .transpose()?
            .unwrap_or_else(|| Vec::new());
        let derivatives = if let Some(d) = derivatives {
            match d.unpack()? {
                Some(DerivativeSpecModes::Set(d)) => Ok(Some(d)),
                Some(DerivativeSpecModes::Discover) => match paths.first() {
                    Some(path) => {
                        if paths.len() > 1 {
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
        if let Some(db_path) = &cache {
            if !reset_cache && db_path.exists() {
                return Self::load_with_roots(paths, derivatives, db_path.to_path_buf());
            }
        }
        let result = Self {
            inner: Layout::create(paths, derivatives, validate)?,
        };
        if let Some(db_path) = cache {
            result.save(db_path)?;
        }
        Ok(result)
    }

    #[getter]
    fn entities(&self) -> PyResult<HashMap<&str, Vec<&String>>> {
        Ok(self.inner.entity_fullkey_vals())
    }

    #[getter]
    fn metadata(&self) -> PyResult<HashMap<&str, Vec<&String>>> {
        self.inner.metadata_key_vals().ok_or_else(|| {
            PyAttributeError::new_err("Metadata must first be indexed by calling .index_metadata()")
        })
    }

    #[getter]
    fn roots(&self) -> Vec<&PathBuf> {
        self.inner.get_roots() //.iter().map(|s| s.to_string_lossy())
    }

    #[getter]
    fn root(&self) -> PyResult<&PathBuf> {
        fn try_with(r: Vec<&PathBuf>) -> PyResult<Option<&PathBuf>> {
            if r.len() > 1 {
                let mut msg = String::from("Layout is multi-root:\n");
                for root in r {
                    msg.push_str(&format!("  {:?}\n", root));
                }
                Err(PyValueError::new_err(msg))
            } else if let Some(root) = r.first() {
                Ok(Some(root))
            } else {
                Ok(None)
            }
        }
        if let Some(root) = try_with(self.inner.get_raw_roots())? {
            Ok(root)
        } else if let Some(root) = try_with(self.inner.get_derivative_roots())? {
            Ok(root)
        } else {
            Err(PyBaseException::new_err(
                "Unexpected problem: no roots found",
            ))
        }
    }

    #[getter]
    fn description(&self) -> PyResult<PyDatasetDescription> {
        fn try_with(
            args: Vec<(&PathBuf, Arc<DatasetDescription>)>,
        ) -> PyResult<Option<Arc<DatasetDescription>>> {
            if args.len() > 1 {
                let mut msg = String::from("Layout is multi-root:\n");
                for root in args {
                    msg.push_str(&format!("  {:?}\n", root.0));
                }
                Err(PyValueError::new_err(msg))
            } else if let Some(root) = args.first() {
                Ok(Some(Arc::clone(&root.1)))
            } else {
                Ok(None)
            }
        }
        if let Some(root) = try_with(self.inner.get_raw_descriptions())? {
            Ok(root.into())
        } else if let Some(root) = try_with(self.inner.get_derivative_descriptions())? {
            Ok(root.into())
        } else {
            Err(PyException::new_err("Unexpected problem: no roots found"))
        }
    }

    #[getter]
    fn derivatives(&self) -> PyResult<Self> {
        let deriv_roots = self
            .inner
            .roots
            .derivative_keys()
            .map(|s| s.to_owned())
            .collect_vec();
        if deriv_roots.len() == 0 {
            return Err(PyValueError::new_err("Layout has no derivatives"));
        }
        Ok(Self {
            inner: self
                .inner
                .query(None, Some(deriv_roots), None)
                .expect("Unexpected error"),
        })
    }

    #[pyo3(signature = (**entities))]
    fn get(&self, entities: Option<QueryParams>) -> PyResult<PyLayout> {
        let entities = entities.map(|entities| entities.unpack()).transpose()?;

        Ok(self.inner.query(entities, None, None).map(Self::from)?)
    }

    #[pyo3(signature = (*, root=None, scope=None))]
    fn filter(&self, root: Option<PathList>, scope: Option<ScopeList>) -> PyResult<PyLayout> {
        // Normalize scope
        let scopes = scope
            .map(|scope| -> PyResult<_> { Ok(self.inner.get_scopes(scope.try_into()?)?) })
            .transpose()?
            .flatten();

        // Normalize Root
        let mut root = root.map(|root| root.unpack()).transpose()?;
        if let Some(scopes) = scopes {
            if let Some(root) = &mut root {
                root.extend(scopes)
            } else {
                root = Some(scopes)
            }
        }

        Ok(self.inner.query(None, root, None).map(Self::from)?)
    }

    fn parse(&self, path: PathBuf) -> PyResult<PyObject> {
        to_pybidspath(self.inner.parse(path)?)
    }

    #[getter]
    fn one(&self) -> PyResult<PyObject> {
        if self.inner.len() == 0 {
            Err(PyValueError::new_err("Layout is empty"))
        } else if self.inner.len() > 1 {
            let mut msg = String::from("Expected one path in layout, but got:\n");
            msg.push_str(&self.inner.fmt_elided_list(5));
            let problem_entities: HashMap<_, _> = self
                .inner
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
            Ok(to_pybidspath(self.inner.get_path(0).unwrap())?)
        }
    }

    fn index_metadata(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner.index_metadata();
        slf
    }

    fn __getitem__(&self, i: usize) -> PyResult<PyObject> {
        match self.inner.get_path(i).map(|path| to_pybidspath(path)) {
            Some(path) => path,
            None => Err(PyKeyError::new_err(format!("Index {} out of range", i))),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        let mut repr = String::from(format!("<BidsLayout (len = {})>\n", self.inner.len()));
        let interesting_entities = HashSet::from(["subject", "session", "run"]);
        let entities = self.inner.entity_fullkey_vals();
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
        repr.push_str(&self.inner.fmt_elided_list(10));
        repr
    }

    fn __iter__(&self) -> LayoutIterator {
        LayoutIterator {
            iter: self.inner.get_paths(),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }

    fn __bool__(&self) -> bool {
        self.inner.len() > 0
    }

    #[classmethod]
    fn load(_cls: &PyType, path: PathBuf) -> PyResult<Self> {
        Ok(Self {
            inner: LayoutCache::load(path)?,
        })
    }

    pub fn save(&self, path: PathBuf) -> PyResult<()> {
        LayoutCache::save(&self.inner, path)?;
        Ok(())
    }

    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.deep_clone(),
        }
    }
}

enum Category {
    Raw,
    Derivative,
    Labelled(String),
}
impl PyLayout {
    fn set_category(layout: &mut Layout, root: &Path, category: Category) -> PyResult<()> {
        let result = match category {
            Category::Raw => layout.roots.set_category(&root, RootCategory::Raw),
            Category::Derivative => layout
                .roots
                .set_category(&root, |d| RootCategory::Derivative(d)),
            Category::Labelled(label) => layout
                .roots
                .set_category(&root, |d| RootCategory::Labelled(label.to_string(), d)),
        };
        match result {
            Some(..) => Ok(()),
            None => Err(PyValueError::new_err(format!(
                "Root '{:?}' not found in cache. All roots must be present within the cache",
                root
            ))),
        }
    }
    pub fn load_with_roots(
        roots: Vec<PathBuf>,
        derivatives: Option<Vec<DerivativeSpec>>,
        db_path: PathBuf,
    ) -> PyResult<Self> {
        let mut layout = LayoutCache::load(db_path)?;
        for root in &roots {
            Self::set_category(&mut layout, &root, Category::Raw)?
        }
        for derivative in derivatives.iter().flatten() {
            if let Some(label) = &derivative.label {
                for root in &derivative.paths {
                    Self::set_category(&mut layout, &root, Category::Labelled(label.to_string()))?
                }
            } else {
                for root in &derivative.paths {
                    Self::set_category(&mut layout, &root, Category::Derivative)?
                }
            }
        }

        let all_roots = roots
            .iter()
            .chain(derivatives.iter().flatten().flat_map(|d| &d.paths))
            .map(|s| s.to_owned())
            .collect_vec();
        Ok(Self {
            inner: layout
                .query(None, Some(all_roots), None)
                .expect("Unexpected Error"),
        })
    }
}

impl From<Layout> for PyLayout {
    fn from(value: Layout) -> Self {
        Self { inner: value }
    }
}
