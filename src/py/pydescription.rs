use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;
use pyo3::prelude::*;

use crate::dataset_description::{DatasetDescription, GeneratedBy, SourceDataset};

#[pyclass(module = "rsbids", name = "GeneratedBy")]
#[derive(Debug, Default, Clone)]
pub struct PyGeneratedBy {
    inner: GeneratedBy,
}

#[pymethods]
impl PyGeneratedBy {
    #[getter]
    fn name(&self) -> &String {
        &self.inner.name
    }
    #[getter]
    fn version(&self) -> Option<&String> {
        self.inner.version.as_ref()
    }
    #[getter]
    fn description(&self) -> Option<&String> {
        self.inner.description.as_ref()
    }
    #[getter]
    fn code_url(&self) -> Option<&String> {
        self.inner.code_url.as_ref()
    }
    #[getter]
    fn container(&self) -> Option<&String> {
        self.inner.container.as_ref()
    }
    fn __repr__(&self) -> String {
        format!("{:#?}", self)
    }
}

impl From<GeneratedBy> for PyGeneratedBy {
    fn from(value: GeneratedBy) -> Self {
        Self { inner: value }
    }
}

#[pyclass(module = "rsbids", name = "SourceDataset")]
#[derive(Debug, Default, Clone)]
pub struct PySourceDataset {
    inner: SourceDataset,
}

#[pymethods]
impl PySourceDataset {
    #[getter]
    fn uri(&self) -> Option<&String> {
        self.inner.uri.as_ref()
    }
    #[getter]
    fn doi(&self) -> Option<&String> {
        self.inner.doi.as_ref()
    }
    #[getter]
    fn version(&self) -> Option<&String> {
        self.inner.version.as_ref()
    }
    fn __repr__(&self) -> String {
        format!("{:#?}", self)
    }
}

impl From<SourceDataset> for PySourceDataset {
    fn from(value: SourceDataset) -> Self {
        Self { inner: value }
    }
}

#[pyclass(module = "rsbids", name = "DatasetDescription")]
#[derive(Debug, Default, Clone)]
pub struct PyDatasetDescription {
    inner: Arc<DatasetDescription>,
}

#[pymethods]
impl PyDatasetDescription {
    #[getter]
    fn name(&self) -> Option<&String> {
        self.inner.name.as_ref()
    }
    #[getter]
    fn bids_version(&self) -> Option<&String> {
        self.inner.bids_version.as_ref()
    }
    #[getter]
    fn hed_version(&self) -> Option<Vec<String>> {
        self.inner.hed_version.clone()
    }
    #[getter]
    fn dataset_links(&self) -> Option<HashMap<String, String>> {
        self.inner.dataset_links.clone()
    }
    #[getter]
    fn dataset_type(&self) -> Option<&String> {
        self.inner.dataset_type.as_ref()
    }
    #[getter]
    fn license(&self) -> Option<&String> {
        self.inner.license.as_ref()
    }
    #[getter]
    fn acknowledgements(&self) -> Option<&String> {
        self.inner.acknowledgements.as_ref()
    }
    #[getter]
    fn how_to_acknowledge(&self) -> Option<&String> {
        self.inner.how_to_acknowledge.as_ref()
    }
    #[getter]
    fn authors(&self) -> Option<Vec<String>> {
        self.inner.authors.clone()
    }
    #[getter]
    fn funding(&self) -> Option<Vec<String>> {
        self.inner.funding.clone()
    }
    #[getter]
    fn ethics_approvals(&self) -> Option<Vec<String>> {
        self.inner.ethics_approvals.clone()
    }
    #[getter]
    fn references_and_links(&self) -> Option<Vec<String>> {
        self.inner.references_and_links.clone()
    }
    #[getter]
    fn dataset_doi(&self) -> Option<&String> {
        self.inner.dataset_doi.as_ref()
    }
    #[getter]
    fn generated_by(&self) -> Option<Vec<PyGeneratedBy>> {
        self.inner
            .generated_by
            .as_ref()
            .map(|g| g.iter().cloned().map_into().collect_vec())
    }
    #[getter]
    fn source_datasets(&self) -> Option<Vec<PySourceDataset>> {
        self.inner
            .source_datasets
            .as_ref()
            .map(|g| g.iter().cloned().map_into().collect_vec())
    }
    #[getter]
    fn pipeline_description(&self) -> Option<PyGeneratedBy> {
        self.inner
            .pipeline_description
            .as_ref()
            .map(|g| g.clone().into())
    }

    fn __repr__(&self) -> String {
        format!("{:#?}", self)
    }
}

impl From<Arc<DatasetDescription>> for PyDatasetDescription {
    fn from(value: Arc<DatasetDescription>) -> Self {
        Self { inner: value }
    }
}
