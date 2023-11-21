use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    iter,
    path::{Path, PathBuf}, sync::Arc,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{apply, serde_as};

use crate::errors::DatasetDescriptionErr;

#[apply(
    Option => #[serde_as(deserialize_as="serde_with::DefaultOnError")] #[serde(default)]
)]
#[serde_with::serde_as]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct GeneratedBy {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "CodeURL")]
    pub code_url: Option<String>,
    #[serde(rename = "container")]
    pub container: Option<String>,
}

#[apply(
    Option => #[serde_as(deserialize_as="serde_with::DefaultOnError")] #[serde(default)]
)]
#[serde_with::serde_as]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SourceDataset {
    #[serde(rename = "URI")]
    pub uri: Option<String>,
    #[serde(rename = "DOI")]
    pub doi: Option<String>,
    #[serde(rename = "Version")]
    pub version: Option<String>,
}

#[apply(
    Option => #[serde_as(deserialize_as="serde_with::DefaultOnError")] #[serde(default)]
)]
#[serde_with::serde_as]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DatasetDescription {
    // #[serde_as(deserialize_as="serde_with::DefaultOnError")]
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "BidsVersion")]
    pub bids_version: Option<String>,
    #[serde(rename = "HEDVersion")]
    pub hed_version: Option<Vec<String>>,
    #[serde(rename = "DatasetLinks")]
    pub dataset_links: Option<HashMap<String, String>>,
    #[serde(rename = "DatasetType")]
    pub dataset_type: Option<String>,
    #[serde(rename = "License")]
    pub license: Option<String>,
    // #[serde_as(deserialize_as="serde_with::DefaultOnError")]
    #[serde(rename = "Authors")]
    pub authors: Option<Vec<String>>,
    #[serde(rename = "Acknowledgments")]
    pub acknowledgements: Option<String>,
    #[serde(rename = "HowToAcknowledge")]
    pub how_to_acknowledge: Option<String>,
    #[serde(rename = "Funding")]
    pub funding: Option<Vec<String>>,
    #[serde(rename = "EthicsApprovals")]
    pub ethics_approvals: Option<Vec<String>>,
    #[serde(rename = "ReferencesAndLinks")]
    pub references_and_links: Option<Vec<String>>,
    #[serde(rename = "DatasetDOI")]
    pub dataset_doi: Option<String>,
    #[serde(rename = "GeneratedBy")]
    pub generated_by: Option<Vec<GeneratedBy>>,
    #[serde(rename = "SourceDatasets")]
    pub source_datasets: Option<Vec<SourceDataset>>,
    #[serde(rename = "PipelineDescription")]
    pub pipeline_description: Option<GeneratedBy>,
}

impl DatasetDescription {
    pub fn open(path: &Path) -> Result<DatasetDescription, DatasetDescriptionErr> {
        if path.as_os_str().is_empty() {
            return DatasetDescription::open(&PathBuf::from("dataset_description.json"));
        }
        if path.is_dir() {
            return DatasetDescription::open(&path.join("dataset_description.json"));
        }
        let mut file = File::open(path).map_err(DatasetDescriptionErr::IoErr)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(DatasetDescriptionErr::IoErr)?;
        serde_json::from_str(&contents).map_err(DatasetDescriptionErr::JsonErr)
    }

    pub fn pipeline_names(&self) -> impl Iterator<Item = &String> {
        vec![
            self.generated_by
                .as_ref()
                .map(|gb| gb.iter().map(|gb| &gb.name).collect::<Vec<&String>>()),
            self.pipeline_description
                .as_ref()
                .map(|pd| iter::once(&pd.name).collect::<Vec<&String>>()),
        ]
        .into_iter()
        .flatten()
        .flatten()
    }
}

impl TryFrom<String> for DatasetDescription {
    type Error = DatasetDescriptionErr;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&value).map_err(DatasetDescriptionErr::JsonErr)
    }
}

impl TryFrom<DatasetDescription> for String {
    type Error = DatasetDescriptionErr;
    fn try_from(value: DatasetDescription) -> Result<Self, Self::Error> {
        serde_json::to_string(&value).map_err(DatasetDescriptionErr::JsonErr)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct GeneratedByBin {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub code_url: Option<String>,
    pub container: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SourceDatasetBin {
    pub uri: Option<String>,
    pub doi: Option<String>,
    pub version: Option<String>,
}

/// DefaultOnError breaks bincode, so keep a seperate, simplified struct for encoding to bin
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DatasetDescriptionBin {
    pub name: Option<String>,
    pub bids_version: Option<String>,
    pub hed_version: Option<Vec<String>>,
    pub dataset_links: Option<HashMap<String, String>>,
    pub dataset_type: Option<String>,
    pub license: Option<String>,
    pub authors: Option<Vec<String>>,
    pub acknowledgements: Option<String>,
    pub how_to_acknowledge: Option<String>,
    pub funding: Option<Vec<String>>,
    pub ethics_approvals: Option<Vec<String>>,
    pub references_and_links: Option<Vec<String>>,
    pub dataset_doi: Option<String>,
    pub generated_by: Option<Vec<GeneratedByBin>>,
    pub source_datasets: Option<Vec<SourceDatasetBin>>,
    pub pipeline_description: Option<GeneratedByBin>,
}

impl From<Arc<DatasetDescription>> for DatasetDescriptionBin {
    fn from(value: Arc<DatasetDescription>) -> Self {
        let value = value.as_ref().clone();
        Self {
            name: value.name,
            bids_version: value.bids_version,
            hed_version: value.hed_version,
            dataset_links: value.dataset_links,
            dataset_type: value.dataset_type,
            license: value.license,
            authors: value.authors,
            acknowledgements: value.acknowledgements,
            how_to_acknowledge: value.how_to_acknowledge,
            funding: value.funding,
            ethics_approvals: value.ethics_approvals,
            references_and_links: value.references_and_links,
            dataset_doi: value.dataset_doi,
            generated_by: value.generated_by.map(|s| s.into_iter().map_into().collect()),
            source_datasets: value.source_datasets.map(|s| s.into_iter().map_into().collect()),
            pipeline_description: value.pipeline_description.map(|s| s.into()),
        }
    }
}

impl From<DatasetDescriptionBin> for Arc<DatasetDescription> {
    fn from(value: DatasetDescriptionBin) -> Self {
        Arc::new(DatasetDescription {
            name: value.name,
            bids_version: value.bids_version,
            hed_version: value.hed_version,
            dataset_links: value.dataset_links,
            dataset_type: value.dataset_type,
            license: value.license,
            authors: value.authors,
            acknowledgements: value.acknowledgements,
            how_to_acknowledge: value.how_to_acknowledge,
            funding: value.funding,
            ethics_approvals: value.ethics_approvals,
            references_and_links: value.references_and_links,
            dataset_doi: value.dataset_doi,
            generated_by: value.generated_by.map(|s| s.into_iter().map_into().collect()),
            source_datasets: value.source_datasets.map(|s| s.into_iter().map_into().collect()),
            pipeline_description: value.pipeline_description.map(|s| s.into()),
        })
    }
}

impl From<SourceDataset> for SourceDatasetBin {
    fn from(value: SourceDataset) -> Self {
        Self {
            uri: value.uri,
            doi: value.doi,
            version: value.version
        }
    }
}

impl From<SourceDatasetBin> for SourceDataset {
    fn from(value: SourceDatasetBin) -> Self {
        Self {
            uri: value.uri,
            doi: value.doi,
            version: value.version
        }
    }
}

impl From<GeneratedBy> for GeneratedByBin {
    fn from(value: GeneratedBy) -> Self {
        Self {
            name: value.name,
            version: value.version,
            description: value.description,
            code_url: value.code_url,
            container: value.container,
        }
    }
}

impl From<GeneratedByBin> for GeneratedBy {
    fn from(value: GeneratedByBin) -> Self {
        Self {
            name: value.name,
            version: value.version,
            description: value.description,
            code_url: value.code_url,
            container: value.container,
        }
    }
}