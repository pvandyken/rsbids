use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    iter,
    path::Path,
};

use json::{object::Object, JsonValue};

pub fn find_dataset_description(path: &Path) -> Option<&Path> {
    for parent in path.ancestors() {
        if parent.join("dataset_description.json").exists() {
            return Some(path);
        }
    }
    None
}

pub enum DatasetDescriptionErr {
    IoErr(io::Error),
    JsonErr(json::JsonError),
}

#[derive(Debug, Default)]
pub struct GeneratedBy {
    name: String,
    version: Option<String>,
    description: Option<String>,
    code_url: Option<String>,
    container: Option<String>,
}
impl GeneratedBy {
    fn parse_list(value: &Object) -> Option<Vec<Self>> {
        Some(match value.get("GeneratedBy")? {
            JsonValue::Array(arr) => arr
                .iter()
                .filter_map(|val| match val {
                    JsonValue::Object(val) => Self::parse_one(val),
                    _ => None,
                })
                .collect(),
            _ => None?,
        })
    }

    fn parse_one(val: &Object) -> Option<Self> {
        Some(GeneratedBy {
            name: extract_string("Name", val)?,
            version: extract_string("Version", val),
            description: extract_string("Description", val),
            code_url: extract_string("CodeURL", val),
            container: extract_string("Container", val),
        })
    }
}
#[derive(Debug, Default)]
pub struct SourceDataset {
    uri: Option<String>,
    doi: Option<String>,
    version: Option<String>,
}
impl SourceDataset {
    fn parse(value: &Object) -> Option<Vec<Self>> {
        let result = GeneratedBy::default();
        Some(match value.get("GeneratedBy")? {
            JsonValue::Array(arr) => arr
                .iter()
                .filter_map(|val| match val {
                    JsonValue::Object(val) => Some(Self {
                        uri: extract_string("URI", val),
                        doi: extract_string("DOI", val),
                        version: extract_string("Version", val),
                    }),
                    _ => None,
                })
                .collect(),
            _ => None?,
        })
    }
}

fn extract_string(key: &str, val: &Object) -> Option<String> {
    Some(val.get(key)?.as_str()?.to_string())
}

fn extract_list(key: &str, val: &Object) -> Option<Vec<String>> {
    Some(match val.get(key)? {
        JsonValue::Array(obj) => obj
            .iter()
            .filter_map(|val| Some(val.as_str()?.to_string()))
            .collect(),
        _ => None?,
    })
}

fn extract_map(key: &str, val: &Object) -> Option<HashMap<String, String>> {
    let mut result = HashMap::new();
    Some(match val.get(key)? {
        JsonValue::Object(obj) => {
            for (key, val) in obj.iter() {
                if let Some(val) = val.as_str() {
                    let key = key.to_string();
                    let val = val.to_string();
                    result.insert(key, val);
                } else {
                    continue;
                }
            }
            result
        }
        _ => None?,
    })
}

#[derive(Debug, Default)]
pub struct DatasetDescription {
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
    pub generated_by: Option<Vec<GeneratedBy>>,
    pub source_datasets: Option<Vec<SourceDataset>>,
    pub pipeline_description: Option<GeneratedBy>,
}

impl DatasetDescription {
    pub fn open(path: &Path) -> Result<DatasetDescription, DatasetDescriptionErr> {
        if path.is_dir() {
            return DatasetDescription::open(&path.join("dataset_description.json"));
        }
        let mut file = File::open(path).map_err(DatasetDescriptionErr::IoErr)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(DatasetDescriptionErr::IoErr)?;
        Ok(
            match json::parse(&contents).map_err(DatasetDescriptionErr::JsonErr)? {
                JsonValue::Object(data) => DatasetDescription {
                    name: extract_string("Name", &data),
                    bids_version: extract_string("BIDSVersion", &data),
                    hed_version: extract_list("HEDVersion", &data),
                    dataset_links: extract_map("DatasetLinks", &data),
                    dataset_type: extract_string("Name", &data),
                    license: extract_string("License", &data),
                    authors: extract_list("Authors", &data),
                    acknowledgements: extract_string("Acknowledgements", &data),
                    how_to_acknowledge: extract_string("HowToAcknowledge", &data),
                    funding: extract_list("Funding", &data),
                    ethics_approvals: extract_list("EthicsApprovals", &data),
                    references_and_links: extract_list("ReferencesAndLinks", &data),
                    dataset_doi: extract_string("DatasetDOI", &data),
                    generated_by: GeneratedBy::parse_list(&data),
                    source_datasets: SourceDataset::parse(&data),
                    pipeline_description: GeneratedBy::parse_one(&data),
                },
                _ => DatasetDescription::default(),
            },
        )
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
