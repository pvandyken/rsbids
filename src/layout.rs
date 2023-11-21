use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsString,
    io,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use once_cell::sync::OnceCell;

use builders::{LayoutBuilder, RootLabel};
pub use iterator::BidsPathViewIterator;
use serde::{Deserialize, Serialize};

use crate::{
    dataset_description::DatasetDescription,
    errors::{BidsPathErr, IterdirErr, QueryErr},
    fs::{iterdir, IterIgnore},
    py::pyparams::derivatives::DerivativeSpec,
    standards::{check_entity, deref_key_alias, get_key_alias, BIDS_DATATYPES},
};

use self::{
    bidspath::BidsPath,
    builders::{
        bidspath_builder::BidsPathBuilder, layout_builder::FileTree,
        metadata_builder::MetadataIndexBuilder,
    },
    entity_table::EntityTable,
    roots::{DatasetRoot, DatasetRoots},
};

pub mod bidspath;
pub mod builders;
pub mod cache;
pub mod entity_table;
pub mod iterator;
pub mod roots;
pub mod utfpath;

pub fn check_datatype(datatype: &str) -> bool {
    BIDS_DATATYPES.contains(datatype)
}

pub fn normalize_query(
    query: HashMap<String, Vec<QueryTerms>>,
) -> HashMap<String, Vec<QueryTerms>> {
    query
        .into_iter()
        .filter_map(|(key, vals)| {
            if vals.len() > 0 {
                let derefed = deref_key_alias(&key)
                    .map(ToString::to_string)
                    .unwrap_or(key);
                let derefed = derefed
                    .strip_suffix("_")
                    .map(ToString::to_string)
                    .unwrap_or(derefed);
                Some((derefed, vals))
            } else {
                None
            }
        })
        .collect()
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub enum QueryTerms {
    Bool(bool),
    String(String),
    Number(u64),
    Any,
}

impl From<&'static str> for QueryTerms {
    fn from(value: &'static str) -> Self {
        QueryTerms::String(value.to_string())
    }
}

impl From<bool> for QueryTerms {
    fn from(value: bool) -> Self {
        QueryTerms::Bool(value)
    }
}

#[macro_export]
macro_rules! construct_query {
    ( $( $key:literal : [ $( $value:expr ),* ] ),* $(,)? ) => {{
        let mut query_map = HashMap::new();
        $(
            query_map.insert($key.to_string(), vec![$crate::layout::QueryTerms::from($value),*]);
        )*
        query_map
    }};

    ( $( $key:literal : $value:expr ),* $(,)? ) => {{
        let mut query_map = HashMap::new();
        $(
            query_map.insert($key.to_string(), vec![$crate::layout::QueryTerms::from($value)]);
        )*
        Some(query_map)
    }};

    ( $( $key:expr => $value:expr ),* $(,)? ) => {{
        let mut query_map = HashMap::new();
        $(
            query_map.insert($key.to_string(), vec![$crate::layout::QueryTerms::from($value)]);
        )*
        Some(query_map)
    }};
}

fn missing_paths_err(msg: String) -> IterdirErr {
    IterdirErr::Io(io::Error::new(io::ErrorKind::NotFound, msg))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Layout {
    paths: Arc<Vec<BidsPath>>,
    entities: EntityTable<String>,
    pub roots: DatasetRoots,
    heads: HashMap<String, HashSet<usize>>,
    filetree: Arc<FileTree>,
    depths: Arc<BTreeMap<usize, HashSet<usize>>>,
    #[serde(
        serialize_with = "crate::serialize::serialize",
        deserialize_with = "crate::serialize::deserialize"
    )]
    metadata: OnceCell<EntityTable<String>>,
    #[serde(
        serialize_with = "crate::serialize::serialize",
        deserialize_with = "crate::serialize::deserialize"
    )]
    view: OnceCell<Vec<usize>>,
}

impl Layout {
    pub fn create(
        paths: Vec<PathBuf>,
        derivatives: Option<Vec<DerivativeSpec>>,
        validate: bool,
    ) -> Result<Layout, IterdirErr> {
        let mut dataset = LayoutBuilder::default();
        let mut invalid_paths = Vec::new();
        if let Some(deriv) = derivatives.as_ref() {
            for d in deriv.iter().flat_map(|d| &d.paths) {
                if !Path::new(&d).exists() {
                    invalid_paths.push(d)
                }
            }
        }
        for path in &paths {
            if !Path::new(&path).exists() {
                invalid_paths.push(&path)
            }
        }
        if invalid_paths.len() > 1 {
            let mut msg = String::from("The following paths do not exist:\n");
            for path in invalid_paths {
                msg.push_str(&format!("  {}\n", path.to_string_lossy()));
            }
            return Err(missing_paths_err(msg));
        } else if let Some(path) = invalid_paths.first() {
            return Err(missing_paths_err(format!(
                "Path does not exist: {}",
                path.to_string_lossy(),
            )));
        }

        let mut ignore = IterIgnore::new();
        ignore.paths.extend(
            paths
                .iter()
                .chain(derivatives.iter().flatten().flat_map(|d| &d.paths))
                .map(|s| PathBuf::from(s)),
        );
        ignore.names = HashSet::from([
            OsString::from("derivatives"),
            OsString::from("sourcedata"),
            OsString::from("code"),
        ]);
        for path in paths {
            let rootpos = dataset
                .register_root(Some(&path), RootLabel::Raw)
                .unwrap_or(0);
            iterdir(path, &ignore, |path| {
                // Ignoring validation errors for now
                dataset.add_path(path, rootpos, validate).unwrap_or(())
            })?;
        }
        if let Some(derivatives) = derivatives {
            for derivative in derivatives {
                let label = match derivative.label {
                    Some(label) => RootLabel::DerivativeLabelled(label),
                    None => RootLabel::DerivativeUnlabelled,
                };
                for path in derivative.paths {
                    let rootpos = dataset
                        .register_root(Some(&path), label.clone())
                        .unwrap_or(0);
                    iterdir(path, &ignore, |path| {
                        // Ignoring validation errors for now
                        dataset.add_path(path, rootpos, validate).unwrap_or(())
                    })?;
                }
            }
        }
        Ok(dataset.finalize())
    }

    pub fn parse(&self, path: PathBuf) -> Result<BidsPath, BidsPathErr> {
        let root = BidsPathBuilder::locate_root(&path)
            .map(|r| r.0)
            .unwrap_or(0);
        let builder = BidsPathBuilder::new(path, root)?;
        builder.template_parse(|s| self.entities.contains_key(s) || check_entity(s))
    }

    fn filter_root<'a>(
        view: &Vec<usize>,
        root: (&'a PathBuf, &'a DatasetRoot),
    ) -> Option<(&'a PathBuf, &'a DatasetRoot)> {
        let (root, ranges) = root;
        if view.iter().any(|i| ranges.contains(i)) {
            Some((root, ranges))
        } else {
            None
        }
    }

    pub fn get_roots(&self) -> Vec<&PathBuf> {
        if let Some(view) = self.view.get() {
            self.roots
                .items()
                .filter_map(|root| Self::filter_root(view, root).map(|r| r.0))
                .collect()
        } else {
            self.roots.keys().collect()
        }
    }

    fn filtered_roots<'a, I: Iterator<Item = (&'a PathBuf, &'a DatasetRoot)> + 'a>(
        &'a self,
        roots: I,
    ) -> Box<dyn Iterator<Item = (&'a PathBuf, &'a DatasetRoot)> + 'a> {
        if let Some(view) = self.view.get() {
            Box::new(roots.filter_map(|root| {
                let (root, ranges) = root;
                if view.iter().any(|i| ranges.contains(i)) {
                    Some((root, ranges))
                } else {
                    None
                }
            }))
        } else {
            Box::new(self.roots.raw_items())
        }
    }

    pub fn get_raw_roots(&self) -> Vec<&PathBuf> {
        self.filtered_roots(self.roots.raw_items())
            .map(|r| r.0)
            .collect()
    }

    pub fn get_derivative_roots(&self) -> Vec<&PathBuf> {
        self.filtered_roots(self.roots.derivative_items())
            .map(|r| r.0)
            .collect()
    }

    pub fn get_raw_descriptions(&self) -> Vec<(&PathBuf, Arc<DatasetDescription>)> {
        self.filtered_roots(self.roots.raw_items())
            .filter_map(|r| r.1.get_description().map(|d| (r.0, d)))
            .collect()
    }

    pub fn get_derivative_descriptions(&self) -> Vec<(&PathBuf, Arc<DatasetDescription>)> {
        self.filtered_roots(self.roots.derivative_items())
            .filter_map(|r| r.1.get_description().map(|d| (r.0, d)))
            .collect()
    }

    pub fn display_root_ranges(&self) -> String {
        format!("{:?}", self.roots)
    }

    pub fn entity_keys(&self) -> impl Iterator<Item = &String> {
        self.entities.keys()
    }

    pub fn entity_vals(&self, key: &str) -> Option<Vec<&String>> {
        self.entities.get(key).map(|val| val.keys().collect_vec())
    }

    pub fn entity_key_vals(&self) -> HashMap<&String, Vec<&String>> {
        self.entities
            .iter()
            .map(|(key, value)| (key, value.keys().collect_vec()))
            .collect()
    }

    pub fn entity_fullkey_vals(&self) -> HashMap<&str, Vec<&String>> {
        self.entities
            .iter()
            .map(|(key, value)| (get_key_alias(key), value.keys().collect_vec()))
            .collect()
    }

    pub fn metadata_key_vals(&self) -> Option<HashMap<&str, Vec<&String>>> {
        self.metadata.get().map(|m| {
            m.iter()
                .map(|(key, value)| (key as &str, value.keys().collect_vec()))
                .collect()
        })
    }

    pub fn fmt_elided_list(&self, limit: usize) -> String {
        let mut msg = String::from("[ ");
        msg.push_str(
            &self
                .get_paths()
                .take(limit)
                .map(|bp| format!("\"{}\"", bp.path.as_str()))
                .join("\n  "),
        );
        if self.len() > limit {
            msg.push_str("\n  ...")
        }
        msg.push_str(" ]");
        msg
    }

    /// Returns the current view on the layout as a vector
    pub fn get_view(&self) -> &Vec<usize> {
        self.view
            .get_or_init(|| self.full_range().into_iter().collect())
    }

    fn full_range(&self) -> Range<usize> {
        0..self.paths.len()
    }

    pub fn all_entity_indices(&self, entity: &str) -> Option<HashSet<usize>> {
        Some(
            self.entities
                .get(entity)?
                .values()
                .fold(HashSet::<usize>::new(), |set, next| &set | next),
        )
    }

    pub fn get_paths(&self) -> BidsPathViewIterator {
        if let Some(_) = self.view.get() {
            BidsPathViewIterator::new(
                Arc::clone(&self.paths),
                self.entity_keys().cloned().collect(),
                Some(self.get_view().clone()),
            )
        } else {
            BidsPathViewIterator::new(
                Arc::clone(&self.paths),
                self.entity_keys().cloned().collect(),
                None,
            )
        }
    }

    pub fn get_path(&self, index: usize) -> Option<BidsPath> {
        let ix = if let Some(view) = self.view.get() {
            *view.iter().nth(index)?
        } else {
            index
        };
        self.paths.get(ix).cloned().map(|mut path| {
            path.update_parents(&self.entity_keys().cloned().collect());
            path
        })
    }

    /// The total number of paths in the layout, ignoring applied views
    pub fn num_paths(&self) -> usize {
        self.paths.len()
    }

    /// The total number of paths in the current view of the layout
    pub fn len(&self) -> usize {
        if let Some(idx) = self.view.get() {
            idx.len()
        } else {
            self.num_paths()
        }
    }

    pub fn get_scopes(&self, scopes: Vec<String>) -> Result<Option<Vec<PathBuf>>, QueryErr> {
        self.roots.get_scopes(scopes)
    }

    fn query_entity(
        &self,
        query: Vec<QueryTerms>,
        entity: &String,
        values: &HashMap<String, HashSet<usize>>,
        new_entities: &mut HashMap<String, HashMap<String, HashSet<usize>>>,
    ) -> Result<HashSet<usize>, QueryErr> {
        let mut new_entity_vals = HashMap::new();
        let mut has_true = false;
        let mut has_false = false;
        let mut queried = HashSet::new();
        for q in query {
            match q {
                QueryTerms::Bool(boolean) => match boolean {
                    true => {
                        has_true = true;
                    }
                    false => {
                        has_false = true;
                    }
                },
                QueryTerms::String(string) => {
                    queried.insert(string);
                }
                QueryTerms::Number(num) => {
                    let matches: HashSet<_> = values
                        .keys()
                        .filter_map(|v| {
                            if v.parse::<u64>() == Ok(num) {
                                Some(v)
                            } else {
                                None
                            }
                        })
                        .collect();
                    if matches.len() > 1 {
                        return Err(QueryErr::AmbiguousQuery(
                            entity.clone(),
                            num,
                            matches.into_iter().cloned().collect(),
                        ));
                    }
                    if let Some(m) = matches.into_iter().next() {
                        queried.insert(m.to_owned());
                    }
                }
                QueryTerms::Any => (),
            }
        }
        let mut selection: HashSet<usize> = values
            .iter()
            .filter_map(|(label, indices)| {
                if queried.remove(label) || has_true {
                    new_entity_vals.insert(label.clone(), indices.clone());
                    Some(indices)
                } else {
                    None
                }
            })
            .fold(HashSet::new(), |set, next| &set | next);
        if has_false {
            let false_indices: HashSet<_> = self
                .get_view()
                .iter()
                .cloned()
                .collect::<HashSet<_>>()
                .difference(&self.all_entity_indices(&entity).unwrap())
                .cloned()
                .collect();
            selection = &selection | &false_indices;
        }
        new_entities.insert(entity.clone(), new_entity_vals);
        if queried.len() > 0 {
            Err(QueryErr::MissingVal(
                entity.clone(),
                queried.into_iter().collect(),
            ))
        } else {
            Ok(selection)
        }
    }

    pub fn query(
        &self,
        query: Option<HashMap<String, Vec<QueryTerms>>>,
        roots: Option<Vec<PathBuf>>,
        mask: Option<&HashSet<usize>>,
    ) -> Result<Layout, QueryErr> {
        let mut new_entities = EntityTable::new();
        let mut new_metadata = EntityTable::new();
        let queried = match query {
            Some(query) => Some({
                // let not_found = Vec::new();
                let mut query = normalize_query(query);
                let mut missing_vals = Vec::new();
                let mut selected = Vec::new();
                for (entity, values) in self.entities.iter() {
                    match query.remove(entity) {
                        Some(queried) => {
                            match self.query_entity(queried, &entity, &values, &mut new_entities) {
                                Ok(ent) => selected.push(ent),
                                Err(err) => missing_vals.push(err),
                            }
                        }
                        None => {
                            new_entities.insert(entity.clone(), values.clone());
                        }
                    }
                }
                let md_selected = if let Some(metadata) = self.metadata.get() {
                    let mut md_selected = Vec::new();
                    for (entity, values) in metadata.iter() {
                        match query.remove(entity) {
                            Some(queried) => {
                                match self.query_entity(
                                    queried,
                                    &entity,
                                    &values,
                                    &mut new_metadata,
                                ) {
                                    Ok(ent) => md_selected.push(ent),
                                    Err(err) => missing_vals.push(err),
                                }
                            }
                            None => {
                                new_entities.insert(entity.clone(), values.clone());
                            }
                        }
                    }
                    Some(md_selected)
                } else {
                    None
                };

                if query.len() > 0 {
                    return Err(QueryErr::MissingEntity(query.keys().cloned().collect()));
                }

                if missing_vals.len() > 0 {
                    // For now ignore value errors
                    // return Err(QueryErr::MutliErr(missing_vals));
                }

                let selected = selected
                    .into_iter()
                    .reduce(|set, next| &set & &next)
                    .unwrap_or_else(|| HashSet::new());

                let md_selected = md_selected.map(|m| {
                    m.into_iter()
                        .reduce(|set, next| &set & &next)
                        .unwrap_or_else(|| HashSet::new())
                });

                if let Some(md_selected) = md_selected {
                    &selected | &md_selected
                } else {
                    selected
                }
            }),
            None => {
                new_entities = self.entities.clone();
                None
            }
        };

        let roots = roots
            .map(|roots| -> Result<_, QueryErr> { Ok(self.roots.glob_roots(roots)?) })
            .transpose()?;

        let root_ranges = roots.as_ref().map(|roots| roots.into_set());

        let selected = vec![mask, root_ranges.as_ref(), queried.as_ref()]
            .into_iter()
            .flatten()
            .fold(None, |set, next| match set {
                Some(s) => Some(&s & next),
                None => Some(next.clone()),
            });

        let filtered_entities: EntityTable<String> = if let Some(selected) = &selected {
            Self::filter_entity_table(new_entities, selected)
        } else {
            new_entities
        };
        let filtered_metadata: EntityTable<String> = if let Some(selected) = &selected {
            Self::filter_entity_table(new_metadata, selected)
        } else {
            new_metadata
        };

        Ok(Layout {
            paths: Arc::clone(&self.paths),
            entities: filtered_entities,
            roots: roots.unwrap_or_else(|| self.roots.clone()),
            heads: self.heads.clone(),
            filetree: Arc::clone(&self.filetree),
            depths: Arc::clone(&self.depths),
            metadata: if self.metadata.get().is_none() {
                OnceCell::new()
            } else {
                OnceCell::with_value(filtered_metadata)
            },
            view: match selected {
                Some(selected) => OnceCell::with_value(selected.into_iter().sorted().collect()),
                None => self.view.clone(),
            },
        })
    }

    /// Filter entity table based on a mask
    fn filter_entity_table(
        table: EntityTable<String>,
        mask: &HashSet<usize>,
    ) -> EntityTable<String> {
        table
            .into_iter()
            .filter_map(|(entity, values)| {
                let filtered_values: HashMap<_, _> = values
                    .into_iter()
                    .filter_map(|(value, insts)| {
                        let new = mask & &insts;
                        if new.len() > 0 {
                            Some((value, new))
                        } else {
                            None
                        }
                    })
                    .collect();
                if filtered_values.len() > 0 {
                    Some((entity, filtered_values))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>()
            .into()
    }

    pub fn index_metadata(&mut self) {
        self.metadata.get_or_init(|| {
            let md_builder =
                MetadataIndexBuilder::build(self.depths.as_ref(), self.filetree.as_ref(), self);
            md_builder.metadata
        });
    }

    pub fn deep_clone(&self) -> Self {
        Self {
            paths: Arc::new(self.paths.as_ref().clone()),
            entities: self.entities.clone(),
            roots: self.roots.clone(),
            heads: self.heads.clone(),
            filetree: Arc::new(self.filetree.as_ref().clone()),
            depths: Arc::new(self.depths.as_ref().clone()),
            metadata: self.metadata.clone(),
            view: self.view.clone(),
        }
    }

}

impl PartialEq for Layout {
    fn eq(&self, other: &Self) -> bool {
        let same_view = || self.get_view() == other.get_view();
        // If both have the same path pointer, check is really quick
        if Arc::ptr_eq(&other.paths, &self.paths) {

            if same_view() {
                true

            } else {
                false
            }
        // Otherwise need exhaustive search
        // Note that root equality is implied by path equality (equal paths must have the same root)
        } else if same_view() {
            let ourpaths: HashSet<_> = self.paths.iter().cloned().collect();
            let theirpaths: HashSet<_> = other.paths.iter().cloned().collect();
            ourpaths == theirpaths
        } else {
            false
        }
    }
}