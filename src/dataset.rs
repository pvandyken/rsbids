use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use once_cell::sync::OnceCell;

use builder::{DatasetBuilder, EntityTable, RootLabel};
pub use iterator::BidsPathViewIterator;

use crate::{
    bidspath::BidsPath,
    fs::{iterdir, iterdir_async},
    pyparams::derivatives::DerivativeSpec,
    standards::{deref_key_alias, get_key_alias, BIDS_DATATYPES},
};

use self::roots::{DatasetRoot, DatasetRoots};

mod builder;
pub mod iterator;
mod roots;

pub fn check_datatype(datatype: &str) -> bool {
    BIDS_DATATYPES.contains(datatype)
}

pub fn normalize_query(
    query: HashMap<String, Vec<QueryTerms>>,
) -> HashMap<String, Vec<QueryTerms>> {
    query
        .into_iter()
        .map(|(key, vals)| {
            let derefed = deref_key_alias(&key)
                .map(ToString::to_string)
                .unwrap_or(key);
            (
                derefed
                    .strip_suffix("_")
                    .map(ToString::to_string)
                    .unwrap_or(derefed),
                vals,
            )
        })
        .collect()
}

#[derive(Eq, PartialEq, Hash)]
pub enum QueryTerms {
    Bool(bool),
    String(String),
}

pub enum QueryErr {
    MissingEntity(Vec<String>),
    MissingVal(String, Vec<String>),
    GlobErr(globset::Error),
}

impl Display for QueryErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryErr::MissingVal(entity, vals) => f.write_fmt(format_args!(
                "For entity: '{}': values not found: [{}]",
                entity,
                vals.iter().map(|val| format!("\"{}\"", val)).join(", ")
            )),
            QueryErr::MissingEntity(entities) => {
                f.write_fmt(format_args!("Entity not found: [{}]", entities.join(", ")))
            }
            QueryErr::GlobErr(err) => f.write_fmt(format_args!("{}", err)),
        }
    }
}

#[derive(Clone)]
pub struct Dataset {
    paths: Arc<Vec<BidsPath>>,
    entities: EntityTable,
    pub roots: DatasetRoots,
    view: OnceCell<HashSet<usize>>,
}

impl Dataset {
    pub fn create(
        paths: Vec<String>,
        derivatives: Option<Vec<DerivativeSpec>>,
        async_walk: bool,
    ) -> Result<Dataset, String> {
        let mut dataset = DatasetBuilder::default();
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
                msg.push_str(&format!("  {}\n", path));
            }
            return Err(msg);
        } else if let Some(path) = invalid_paths.first() {
            return Err(format!("Path does not exist: {}", path));
        }
        for path in paths {
            let rootpos = dataset
                .register_root(Some(&path), RootLabel::Raw)
                .unwrap_or(0);
            match (if async_walk { iterdir_async } else { iterdir })(PathBuf::from(path), |path| {
                dataset.add_path(path, rootpos)
            }) {
                Ok(..) => Ok(()),
                Err(e) => Err(format!("{}", e)),
            }?;
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
                    match iterdir(PathBuf::from(path), |path| dataset.add_path(path, rootpos)) {
                        Ok(..) => Ok(()),
                        Err(e) => {
                            dbg!(&e);
                            Err(format!("{}", e))
                        }
                    }?;
                }
            }
        }
        Ok(dataset.finalize())
    }

    fn filter_root<'a>(
        view: &HashSet<usize>,
        root: (&'a String, &DatasetRoot),
    ) -> Option<&'a String> {
        let (root, ranges) = root;
        if view.iter().any(|i| ranges.contains(i)) {
            Some(root)
        } else {
            None
        }
    }

    pub fn get_roots(&self) -> Vec<&String> {
        if let Some(view) = self.view.get() {
            self.roots
                .items()
                .filter_map(|root| Self::filter_root(view, root))
                .collect()
        } else {
            self.roots.keys().collect()
        }
    }

    pub fn get_raw_roots(&self) -> Vec<&String> {
        if let Some(view) = self.view.get() {
            self.roots
                .raw_items()
                .filter_map(|root| Self::filter_root(view, root))
                .collect()
        } else {
            self.roots.raw_keys().collect()
        }
    }

    pub fn get_derivative_roots(&self) -> Vec<&String> {
        if let Some(view) = self.view.get() {
            self.roots
                .derivative_items()
                .filter_map(|root| Self::filter_root(view, root))
                .collect()
        } else {
            self.roots.derivative_keys().collect()
        }
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

    pub fn fmt_elided_list(&self, limit: usize) -> String {
        let mut msg = String::from("[ ");
        msg.push_str(
            &self
                .get_paths()
                .take(limit)
                .map(|bp| format!("\"{}\"", bp.path))
                .join("\n  "),
        );
        if self.len() > limit {
            msg.push_str("\n  ...")
        }
        msg.push_str(" ]");
        msg
    }

    pub fn all_indices(&self) -> &HashSet<usize> {
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
                Some(self.all_indices().clone()),
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
        self.paths.get(index).cloned().map(|mut path| {
            path.update_parents(&self.entity_keys().cloned().collect());
            path
        })
    }

    pub fn num_paths(&self) -> usize {
        self.paths.len()
    }

    pub fn len(&self) -> usize {
        if let Some(idx) = self.view.get() {
            idx.len()
        } else {
            self.num_paths()
        }
    }

    pub fn get_scopes(&self, scopes: Vec<String>) -> Result<Option<Vec<String>>, QueryErr> {
        self.roots.get_scopes(scopes)
    }

    fn query_entity(
        &self,
        query: Vec<QueryTerms>,
        entity: &String,
        values: &HashMap<String, HashSet<usize>>,
        new_entities: &mut HashMap<String, HashMap<String, HashSet<usize>>>,
    ) -> Result<Option<HashSet<usize>>, QueryErr> {
        let mut new_entity_vals = HashMap::new();
        let mut has_true = false;
        let mut has_false = false;
        let mut queried: HashSet<_> = query
            .into_iter()
            .filter_map(|query| match query {
                QueryTerms::Bool(boolean) => match boolean {
                    true => {
                        has_true = true;
                        None
                    }
                    false => {
                        has_false = true;
                        None
                    }
                },
                QueryTerms::String(string) => Some(string),
            })
            .collect();
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
                .all_indices()
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
            Ok(Some(selection))
        }
    }

    pub fn query(
        &self,
        query: Option<HashMap<String, Vec<QueryTerms>>>,
        roots: Option<Vec<String>>,
    ) -> Result<Dataset, QueryErr> {
        let mut new_entities = HashMap::new();
        let queried = match query {
            Some(query) => Some({
                let mut query = normalize_query(query);
                let selected = self
                    .entities
                    .iter()
                    .map(|(entity, values)| match query.remove(entity) {
                        Some(queried) => {
                            self.query_entity(queried, entity, values, &mut new_entities)
                        }
                        None => {
                            new_entities.insert(entity.clone(), values.clone());
                            Ok(None)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>();

                if query.len() > 0 {
                    return Err(QueryErr::MissingEntity(query.keys().cloned().collect()));
                }

                selected?
                    .into_iter()
                    .flatten()
                    .reduce(|set, next| &set & &next)
                    .unwrap_or_else(|| HashSet::new())
            }),
            None => {
                new_entities = self.entities.clone();
                None
            }
        };

        let roots = roots
            .map(|roots| Ok(self.roots.glob_roots(roots).map_err(QueryErr::GlobErr)?))
            .transpose()?;

        let root_ranges = roots.as_ref().map(|roots| roots.full_range());

        let selected = vec![root_ranges, queried]
            .into_iter()
            .flatten()
            .reduce(|set, next| &set & &next);

        let filtered_entities: HashMap<_, _> = if let Some(selected) = &selected {
            new_entities
                .into_iter()
                .filter_map(|(entity, values)| {
                    let filtered_values: HashMap<_, _> = values
                        .into_iter()
                        .filter_map(|(value, insts)| {
                            let new = selected & &insts;
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
                .collect()
        } else {
            new_entities
        };

        Ok(Dataset {
            paths: Arc::clone(&self.paths),
            entities: filtered_entities,
            roots: roots.unwrap_or_else(|| self.roots.clone()),
            view: match selected {
                Some(selected) => OnceCell::with_value(selected),
                None => OnceCell::new(),
            },
        })
    }
}
