use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsString,
    mem,
    ops::Range,
    path::{Components, Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::{
    errors::BidsPathErr,
    layout::{
        bidspath::{BidsPath, UnknownDatatype, UnknownDatatypeTypes},
        entity_table::EntityTable,
        roots::{DatasetRoot, RootCategory},
        Layout,
    },
    standards::BIDS_ENTITIES,
    utils::is_subpath_of,
};

use super::bidspath_builder::BidsPathBuilder;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileTree {
    nodes: HashMap<OsString, Box<FileTree>>,
    files: HashSet<usize>,
}

impl FileTree {
    pub fn insert(&mut self, mut path: Components, file: usize) {
        if let Some(next) = path.next() {
            let next = next.as_os_str();
            let sub = self.nodes.entry(next.to_owned()).or_default();
            sub.insert(path, file);
        } else {
            self.files.insert(file);
        }
    }

    #[inline]
    pub fn find(&self, path: &Path) -> Option<&FileTree> {
        self.find_impl(path.components())
    }

    #[inline]
    pub fn get_subfiles(&self, path: &Path) -> Option<HashSet<usize>> {
        let tree = self.find(path)?;
        let mut result = HashSet::new();
        tree.get_subfiles_impl(&mut result);
        Some(result)
    }

    fn find_impl(&self, mut path: Components) -> Option<&FileTree> {
        if let Some(next) = path.next() {
            let next = next.as_os_str();
            if let Some(sub) = self.nodes.get(next) {
                sub.find_impl(path)
            } else {
                None
            }
        } else {
            Some(self)
        }
    }

    fn get_subfiles_impl(&self, result: &mut HashSet<usize>) {
        for tree in self.nodes.values() {
            tree.get_subfiles_impl(result);
        }
        result.extend(self.files.iter().cloned());
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
    Raw(PathBuf, Range<usize>),
    Derivative(PathBuf, Option<String>, Range<usize>),
}


#[derive(Debug, Default, Clone)]
pub struct LayoutBuilder {
    paths: Vec<BidsPath>,
    pub(super) entities: EntityTable<String>,
    roots: HashMap<PathBuf, DatasetRoot>,
    derivative_roots: HashMap<PathBuf, DatasetRoot>,
    labelled_roots: HashMap<String, HashMap<PathBuf, DatasetRoot>>,
    pub(super) heads: HashMap<String, HashSet<usize>>,
    pub(super) depths: BTreeMap<usize, HashSet<usize>>,
    pub(super) filetree: FileTree,
    current_root: Option<PartialRoot>,
    unknown_entities: EntityTable<String>,
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

    pub(super) fn add_depth(&mut self, depth: usize) {
        let i = self.current_path();
        if let Some(val) = self.depths.get_mut(&depth) {
            val.insert(i);
        } else {
            self.depths.insert(depth, HashSet::from([i]));
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


    fn merge_path(&mut self, path: &BidsPath) {
        let i = self.current_path();
        for (entity, vals) in path.get_entities() {
            self.entities.insert_entity(i, entity, vals)
        }
        if let Some(uncertain_entities) = path.get_uncertain_entities() {
            for (entity, vals) in uncertain_entities {
                self.add_entity(&entity, vals);
            }
        }
        if !path.uncertain_datatypes.is_none() {
            self.add_uncertain_datatype()
        }
    }

    pub fn register_root(&mut self, root: Option<&PathBuf>, label: RootLabel) -> Option<usize> {
        // Paths here come from user input, so safe to use to_string_lossy throughout
        let (len, root) = root
            .map(|r| BidsPathBuilder::locate_root(r))
            .flatten()
            .map(|(len, path)| (Some(len), Some(path.to_owned())))
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

    fn add_raw_root(&mut self, root: PathBuf, mut range: Range<usize>) {
        range.end = self.paths.len();
        Self::insert_to_root_map(&mut self.roots, root, range);
    }

    fn add_derivative_root(
        &mut self,
        root: PathBuf,
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
        map: &mut HashMap<PathBuf, DatasetRoot>,
        key: PathBuf,
        range: Range<usize>,
    ) {
        if let Some(entry) = map.get_mut(&key) {
            entry.insert(range);
        } else {
            let new_root = DatasetRoot::new_range(range, Some(&Path::new(&key)));
            map.insert(key, new_root);
        }
    }

    pub fn add_path(
        &mut self,
        path: PathBuf,
        root: usize,
        with_spec: bool,
    ) -> Result<(), BidsPathErr> {
        let pathbuf = PathBuf::from(&path);
        let mut pathcomps = pathbuf.components();
        pathcomps.next_back();
        let builder = BidsPathBuilder::new(path, root)?;
        let path = if with_spec {
            let path = builder.spec_parse()?;
            self.merge_path(&path);
            path
        } else {
            builder.generic_build_parse(self)
        };
        self.filetree.insert(pathcomps, self.current_path());
        self.add_head(&path.get_head());
        self.add_depth(path.depth);
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
            heads: self.heads,
            filetree: Arc::new(self.filetree),
            depths: Arc::new(self.depths),
            metadata: OnceCell::new(),
            view: OnceCell::new(),
        }
    }

    fn normalize_roots(
        heads: &Vec<String>,
        roots: HashMap<PathBuf, DatasetRoot>,
    ) -> HashMap<PathBuf, DatasetRoot> {
        let mut result: HashMap<PathBuf, DatasetRoot> = HashMap::new();
        for (root, data) in roots {
            let mut longest_head: Option<&String> = None;
            for head in heads {
                if is_subpath_of(&PathBuf::from(&head), &root) {
                    longest_head = None;
                    break;
                } else if is_subpath_of(&root, &PathBuf::from(&head))
                    && head.len() > longest_head.map(|h| h.len()).unwrap_or(0)
                {
                    longest_head = Some(head)
                }
            }
            match longest_head {
                Some(head) => {
                    let head = PathBuf::from(head);
                    if let Some(droot) = result.get_mut(&head) {
                        droot.extend(data.get_range());
                    } else {
                        result.insert(head, data.move_range().into());
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
