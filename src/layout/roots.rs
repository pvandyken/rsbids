use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    path::Path,
    sync::Arc,
};

use globset::{Glob, GlobSetBuilder};
use itertools::Itertools;

use crate::dataset_description::DatasetDescription;

use super::{builders::primitives::MultiRange, QueryErr};

#[derive(Clone, Debug)]
pub enum RootType<I> {
    DatasetRoot(Arc<DatasetDescription>, I),
    SeedRoot(I),
}

#[derive(Clone, Debug)]
pub struct DatasetRoot {
    roottype: RootType<MultiRange<usize>>,
}

impl DatasetRoot {
    pub fn new_range(range: Range<usize>, desc_path: Option<&Path>) -> Self {
        let description = desc_path
            .map(|p| DatasetDescription::open(p))
            .transpose()
            // Ignoring opening errors for now
            .unwrap_or(None);
        Self {
            roottype: match description {
                Some(desc) => RootType::DatasetRoot(Arc::new(desc), range.into()),
                None => RootType::SeedRoot(range.into()),
            },
        }
    }
    pub fn get_range(&self) -> &MultiRange<usize> {
        match &self.roottype {
            RootType::DatasetRoot(_, ranges) => ranges,
            RootType::SeedRoot(ranges) => ranges,
        }
    }

    pub fn move_range(self) -> MultiRange<usize> {
        match self.roottype {
            RootType::DatasetRoot(_, ranges) => ranges,
            RootType::SeedRoot(ranges) => ranges,
        }
    }
    pub fn contains(&self, i: &usize) -> bool {
        match &self.roottype {
            RootType::DatasetRoot(_, ranges) => ranges.contains(i),
            RootType::SeedRoot(ranges) => ranges.contains(i),
        }
    }

    pub fn insert(&mut self, i: Range<usize>) -> bool {
        match &mut self.roottype {
            RootType::DatasetRoot(_, ref mut ranges) => ranges.insert(i),
            RootType::SeedRoot(ref mut ranges) => ranges.insert(i),
        }
    }

    pub fn extend(&mut self, i: &MultiRange<usize>) {
        match &mut self.roottype {
            RootType::DatasetRoot(_, ref mut ranges) => ranges.extend(i),
            RootType::SeedRoot(ref mut ranges) => ranges.extend(i),
        }
    }
}

impl Into<HashSet<usize>> for &DatasetRoot {
    fn into(self) -> HashSet<usize> {
        match &self.roottype {
            RootType::DatasetRoot(_, ranges) => ranges.into(),
            RootType::SeedRoot(ranges) => ranges.into(),
        }
    }
}

impl From<MultiRange<usize>> for DatasetRoot {
    fn from(value: MultiRange<usize>) -> Self {
        Self {
            roottype: RootType::SeedRoot(value),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RootCategory {
    Raw(DatasetRoot),
    Derivative(DatasetRoot),
    Labelled(String, DatasetRoot),
}

#[derive(Clone, Debug)]
pub struct DatasetRoots {
    roots: HashMap<String, RootCategory>,
}

impl DatasetRoots {
    pub fn get_scopes(&self, scopes: Vec<String>) -> Result<Option<Vec<String>>, QueryErr> {
        let mut result = Vec::new();
        let mut errs = Vec::new();
        for scope in scopes {
            if scope == "raw" || scope == "self" {
                result.extend(self.raw_keys())
            } else if scope == "derivatives" {
                result.extend(self.derivative_keys())
            } else if scope == "all" {
                return Ok(None);
            } else if let Some(labelled) = self.get_by_label(&scope) {
                result.extend(labelled);
            } else if let Some(pipelines) = self.get_by_pipeline(&scope) {
                result.extend(pipelines)
            } else {
                errs.push(scope)
            }
        }
        if errs.len() > 0 {
            Err(QueryErr::MissingVal(String::from("scope"), errs))
        } else {
            Ok(Some(result.iter_mut().map(|s| s.clone()).collect()))
        }
    }
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.roots.keys()
    }

    pub fn items(&self) -> impl Iterator<Item = (&String, &DatasetRoot)> {
        self.roots.iter().map(|(root, data)| match data {
            RootCategory::Derivative(ranges)
            | RootCategory::Raw(ranges)
            | RootCategory::Labelled(_, ranges) => (root, ranges),
        })
    }

    pub fn raw_items(&self) -> impl Iterator<Item = (&String, &DatasetRoot)> {
        self.roots.iter().filter_map(|(root, data)| match data {
            RootCategory::Raw(ranges) => Some((root, ranges)),
            _ => None,
        })
    }

    pub fn derivative_items(&self) -> impl Iterator<Item = (&String, &DatasetRoot)> {
        self.roots.iter().filter_map(|(root, data)| match data {
            RootCategory::Derivative(ranges) | RootCategory::Labelled(_, ranges) => {
                Some((root, ranges))
            }
            _ => None,
        })
    }

    pub fn raw_keys(&self) -> impl Iterator<Item = &String> {
        self.roots.iter().filter_map(|(root, data)| match data {
            RootCategory::Raw(..) => Some(root),
            _ => None,
        })
    }

    pub fn derivative_keys(&self) -> impl Iterator<Item = &String> {
        self.roots.iter().filter_map(|(root, data)| match data {
            RootCategory::Derivative(..) | RootCategory::Labelled(..) => Some(root),
            _ => None,
        })
    }

    pub fn get_by_label<'a>(&'a self, query: &str) -> Option<Vec<&String>> {
        let result = self
            .roots
            .iter()
            .filter_map(move |(root, data)| match data {
                RootCategory::Labelled(label, _) => {
                    if query == label {
                        Some(root)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect_vec();
        if result.len() > 0 {
            Some(result)
        } else {
            None
        }
    }

    pub fn get_by_pipeline<'a>(&'a self, query: &String) -> Option<Vec<&String>> {
        let result = self
            .roots
            .iter()
            .filter_map(|(root, data)| match data {
                RootCategory::Raw(ranges)
                | RootCategory::Derivative(ranges)
                | RootCategory::Labelled(_, ranges) => match &ranges.roottype {
                    RootType::DatasetRoot(desc, _) => {
                        if desc.pipeline_names().contains(query) {
                            Some(root)
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
            })
            .collect_vec();
        if result.len() > 0 {
            Some(result)
        } else {
            None
        }
    }

    fn ranges(&self) -> impl Iterator<Item = &DatasetRoot> {
        self.roots.iter().map(|(_, data)| match data {
            RootCategory::Derivative(ranges)
            | RootCategory::Labelled(_, ranges)
            | RootCategory::Raw(ranges) => ranges,
        })
    }

    pub fn full_range(&self) -> HashSet<usize> {
        self.ranges()
            .map_into()
            .reduce(|set, next| &set | &next)
            .unwrap_or(HashSet::new())
    }

    pub fn glob_roots(&self, roots: Vec<String>) -> Result<Self, globset::Error> {
        let mut builder = GlobSetBuilder::new();
        let mut exact = HashSet::new();

        // Do exact match checks to avoid globbing on paths with potentially invalid glob syntax
        for root in roots {
            if self.roots.contains_key(&root) {
                exact.insert(root);
            } else {
                builder.add(Glob::new(&root)?);
            }
        }
        let glob = builder.build()?;
        Ok(self
            .roots
            .iter()
            .filter_map(|(root, data)| match data {
                RootCategory::Derivative(ranges) => {
                    if exact.contains(root) || glob.is_match(root) {
                        Some((root.clone(), RootCategory::Derivative(ranges.clone())))
                    } else {
                        None
                    }
                }
                RootCategory::Raw(ranges) => {
                    if exact.contains(root) || glob.is_match(root) {
                        Some((root.clone(), RootCategory::Raw(ranges.clone())))
                    } else {
                        None
                    }
                }
                RootCategory::Labelled(label, ranges) => {
                    if exact.contains(root) || glob.is_match(root) {
                        Some((
                            root.clone(),
                            RootCategory::Labelled(label.clone(), ranges.clone()),
                        ))
                    } else {
                        None
                    }
                }
            })
            .collect())
    }
}

impl From<HashMap<String, RootCategory>> for DatasetRoots {
    fn from(value: HashMap<String, RootCategory>) -> Self {
        Self { roots: value }
    }
}

impl FromIterator<(String, RootCategory)> for DatasetRoots {
    fn from_iter<T: IntoIterator<Item = (String, RootCategory)>>(iter: T) -> Self {
        Self {
            roots: iter.into_iter().collect(),
        }
    }
}
