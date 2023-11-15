use std::{
    collections::{HashMap, HashSet},
    mem,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use once_cell::sync::OnceCell;

use crate::{
    layout::{
        entity_table::EntityTable,
        roots::{DatasetRoot, RootCategory},
        Layout, bidspath::{BidsPath, UnknownDatatypeTypes, UnknownDatatype},
    },
    dataset_description::find_dataset_description,
    standards::BIDS_ENTITIES,
};

use super::bidspath_builder::BidsPathBuilder;

trait EntityTableExt {
    fn insert_entity(&mut self, i: usize, entity: &str, value: &str);
}

impl EntityTableExt for EntityTable {
    fn insert_entity(&mut self, i: usize, entity: &str, value: &str) {
        if let Some(val_map) = self.get_mut(entity) {
            if let Some(set) = val_map.get_mut(value) {
                set.insert(i);
            } else {
                val_map.insert(value.to_string(), HashSet::from([i]));
            }
        } else {
            let mut val_map = HashMap::new();
            val_map.insert(value.to_string(), HashSet::from([i]));
            self.insert(entity.to_string(), val_map);
        }
    }
}

#[derive(Debug, Clone)]
pub enum RootLabel {
    Raw,
    DerivativeUnlabelled,
    DerivativeLabelled(String),
}

#[derive(Debug, Clone)]
enum PartialRoot {
    Raw(String, Range<usize>),
    Derivative(String, Option<String>, Range<usize>),
}

#[derive(Debug, Default, Clone)]
pub struct LayoutBuilder {
    paths: Vec<BidsPath>,
    pub(super) entities: EntityTable,
    roots: HashMap<String, DatasetRoot>,
    derivative_roots: HashMap<String, DatasetRoot>,
    labelled_roots: HashMap<String, HashMap<String, DatasetRoot>>,
    pub(super) heads: HashMap<String, HashSet<usize>>,
    current_root: Option<PartialRoot>,
    unknown_entities: EntityTable,
    unknown_datatypes: HashSet<usize>,
}

impl LayoutBuilder {
    fn current_path(&self) -> usize {
        self.paths.len()
    }

    pub(super) fn add_entity(&mut self, entity: &str, value: &str) {
        let i = self.current_path();
        if self.check_entity(entity) {
            self.entities.insert_entity(i, entity, value)
        } else {
            self.unknown_entities.insert_entity(i, entity, value)
        }
    }

    pub(super) fn add_head(&mut self, head: &str) {
        let i = self.current_path();
        if let Some(val) = self.heads.get_mut(head) {
            val.insert(i);
        } else {
            self.heads.insert(head.to_string(), HashSet::from([i]));
        }
    }

    fn confirm_entity(&mut self, entity: &str) {
        if let Some((entity, value)) = self.unknown_entities.remove_entry(entity) {
            self.entities.insert(entity, value);
        }
    }
    pub(super) fn add_and_confirm_entity(&mut self, entity: &str, value: &str) {
        self.confirm_entity(entity);
        self.entities
            .insert_entity(self.current_path(), entity, value)
    }
    pub(super) fn check_entity(&self, entity: &str) -> bool {
        self.entities.contains_key(entity) || BIDS_ENTITIES.contains_left(entity)
    }

    pub(super) fn add_uncertain_datatype(&mut self) {
        self.unknown_datatypes.insert(self.current_path());
    }

    pub fn register_root(&mut self, root: Option<&String>, label: RootLabel) -> Option<usize> {
        let (len, root) = root
            .map(|path| {
                let len = path.len();
                let path = PathBuf::from(path);
                if let Some(description_path) = find_dataset_description(&path) {
                    let description_path = description_path.to_string_lossy();
                    let len = description_path.len();
                    Some((len, description_path.to_string()))
                } else if path.is_file() {
                    if let Some(rootpath) = path.parent() {
                        let rootpath = rootpath.to_string_lossy();
                        let len = rootpath.len();
                        Some((len, rootpath.to_string()))
                    } else {
                        None
                    }
                } else {
                    Some((len, path.to_string_lossy().to_string()))
                }
            })
            .flatten()
            .map(|(len, path)| (Some(len), Some(path)))
            .unwrap_or((None, None));

        // Holding ground for new root, as we don't know the extent of it's range
        let new_range = self.paths.len()..0;
        let mut new_root = root.map(|root| match label {
            RootLabel::DerivativeLabelled(label) => {
                PartialRoot::Derivative(root, Some(label), new_range)
            }
            RootLabel::DerivativeUnlabelled => PartialRoot::Derivative(root, None, new_range),
            RootLabel::Raw => PartialRoot::Raw(root, new_range),
        });
        mem::swap(&mut self.current_root, &mut new_root);

        // Current position marks the end of the last root, so add it to official list
        let prev_root = new_root;
        if let Some(prev_root) = prev_root {
            match prev_root {
                PartialRoot::Derivative(root, label, range) => {
                    self.add_derivative_root(root, label, range)
                }
                PartialRoot::Raw(root, range) => self.add_raw_root(root, range),
            };
        };
        len
    }

    fn add_raw_root(&mut self, root: String, mut range: Range<usize>) {
        range.end = self.paths.len();
        Self::insert_to_root_map(&mut self.roots, root, range);
    }

    fn add_derivative_root(
        &mut self,
        root: String,
        label: Option<String>,
        mut range: Range<usize>,
    ) {
        range.end = self.paths.len();
        match label {
            Some(label) => {
                if let Some(mut map) = self.labelled_roots.get_mut(&label) {
                    Self::insert_to_root_map(&mut map, root, range);
                } else {
                    let new_root = DatasetRoot::new_range(range, Some(Path::new(&root)));
                    self.labelled_roots
                        .insert(label, HashMap::from([(root, new_root)]));
                }
            }
            None => Self::insert_to_root_map(&mut self.derivative_roots, root, range),
        }
    }

    fn insert_to_root_map(
        map: &mut HashMap<String, DatasetRoot>,
        key: String,
        range: Range<usize>,
    ) {
        if let Some(entry) = map.get_mut(&key) {
            entry.insert(range);
        } else {
            let new_root = DatasetRoot::new_range(range, Some(&Path::new(&key)));
            map.insert(key, new_root);
        }
    }

    pub fn add_path(&mut self, path: String, root: usize, with_spec: bool) -> Result<(), BidsPath> {
        let builder = BidsPathBuilder::new(path, root);
        let path = if with_spec {
            builder.spec_parse(Some(self)).map_err(|builder| builder.no_parse())?
        } else {
            builder.generic_build_parse(self)
        };
        self.paths.push(path);
        Ok(())
    }

    fn first_valid_datatype(
        &self,
        uncertain_datatypes: &mut Vec<UnknownDatatypeTypes>,
    ) -> Option<UnknownDatatype> {
        while let Some(dt) = uncertain_datatypes.pop() {
            match dt {
                UnknownDatatypeTypes::Linked(entity, dt) => {
                    if self.check_entity(&entity) || dt.is_valid {
                        return Some(dt);
                    }
                }
                UnknownDatatypeTypes::Unlinked(dt) => {
                    if dt.is_valid {
                        return Some(dt);
                    }
                }
            }
        }
        None
    }

    fn extract_uncertain_datatypes(&mut self, i: usize) -> Option<Vec<UnknownDatatypeTypes>> {
        let path = &mut self.paths[i];
        let mut datatypes = None;
        std::mem::swap(&mut path.uncertain_datatypes, &mut datatypes);
        datatypes
    }

    pub fn finalize(mut self) -> Layout {
        self.register_root(None, RootLabel::Raw);
        let unknown_datatypes = self.unknown_datatypes.drain().collect_vec();
        for i in unknown_datatypes {
            let mut datatypes = self.extract_uncertain_datatypes(i);
            if let Some(datatypes) = datatypes.as_mut() {
                if let Some(dt) = self.first_valid_datatype(datatypes) {
                    self.paths[i].datatype = Some(dt.value)
                }
                while let Some(dt) = datatypes.pop() {
                    match dt {
                        UnknownDatatypeTypes::Linked(_, dt) => self.paths[i].push_part(dt.value),
                        UnknownDatatypeTypes::Unlinked(dt) => self.paths[i].push_part(dt.value),
                    }
                }
            }
        }
        let heads = self
            .heads
            .keys()
            .map(|head| format!("{}{}", head, std::path::MAIN_SEPARATOR_STR))
            .collect_vec();
        let mut roots = HashMap::new();
        roots.extend(
            Self::normalize_roots(&heads, self.roots)
                .into_iter()
                .map(|(key, val)| (key, RootCategory::Raw(val))),
        );
        roots.extend(
            Self::normalize_roots(&heads, self.derivative_roots)
                .into_iter()
                .map(|(key, val)| (key, RootCategory::Derivative(val))),
        );
        roots.extend(self.labelled_roots.into_iter().flat_map(|(label, val)| {
            Self::normalize_roots(&heads, val)
                .into_iter()
                .map(move |(root, val)| (root, RootCategory::Labelled(label.clone(), val)))
        }));
        Layout {
            paths: Arc::new(self.paths),
            entities: self.entities,
            roots: roots.into(),
            view: OnceCell::new(),
        }
    }

    fn normalize_roots(
        heads: &Vec<String>,
        roots: HashMap<String, DatasetRoot>,
    ) -> HashMap<String, DatasetRoot> {
        let mut result: HashMap<String, DatasetRoot> = HashMap::new();
        for (root, data) in roots {
            match heads
                .iter()
                .find(|&head| root.starts_with(head) && root.len() > head.len())
            {
                Some(head) => {
                    if let Some(droot) = result.get_mut(head) {
                        droot.extend(data.get_range());
                    } else {
                        result.insert(head.to_string(), data.move_range().into());
                    }
                }
                None => {
                    result.insert(root, data);
                }
            }
        }
        result
    }

}
