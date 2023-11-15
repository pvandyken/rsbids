use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use itertools::chain;

use crate::{
    primitives::{ComponentType, KeyVal},
    standards::get_key_alias,
};

#[derive(Debug, Clone)]
pub struct UnknownDatatype {
    pub value: Range<usize>,
    pub is_valid: bool,
}

impl UnknownDatatype {
    pub fn new(comp: Range<usize>, is_valid: bool) -> UnknownDatatype {
        UnknownDatatype {
            value: comp,
            is_valid,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnknownDatatypeTypes {
    Linked(String, UnknownDatatype),
    Unlinked(UnknownDatatype),
}

#[derive(Debug, Clone)]
pub struct BidsPath {
    pub path: String,
    pub entities: Vec<KeyVal>,
    pub parts: Option<Vec<Range<usize>>>,
    pub suffix: Option<Range<usize>>,
    pub extension: Option<Range<usize>>,
    pub datatype: Option<Range<usize>>,
    pub parents: Vec<KeyVal>,
    pub head: usize,
    pub root: usize,
    pub uncertain_parents: Option<Vec<KeyVal>>,
    pub uncertain_datatypes: Option<Vec<UnknownDatatypeTypes>>,
}

pub struct BidsPathComponents {
    pub components: Vec<ComponentType>,
}

impl BidsPath {
    pub fn new(path: String, root: usize) -> BidsPath {
        BidsPath {
            path: path,
            head: 0,
            entities: vec![],
            parts: None,
            suffix: None,
            extension: None,
            datatype: None,
            parents: vec![],
            root: root,
            uncertain_parents: None,
            uncertain_datatypes: None,
        }
    }

    pub fn add_uncertain_parent(&mut self, keyval: KeyVal) {
        if let Some(up) = self.uncertain_parents.as_mut() {
            up.push(keyval)
        } else {
            self.uncertain_parents = Some(vec![keyval])
        }
    }

    pub fn update_parents(&mut self, parents: &HashSet<String>) -> Option<()> {
        if self.uncertain_parents.is_none() {
            return None;
        }
        for parent in self.uncertain_parents.as_mut()?.drain(..) {
            let key = parent.get_key(&self.path);
            if parents.contains(key) {
                self.parents.push(parent)
            }
        }
        self.uncertain_parents = None;
        Some(())
    }

    pub fn get_entities(&self) -> HashMap<&str, &str> {
        let mut entities = HashMap::new();
        for parent in chain![&self.parents, &self.entities] {
            let (key, val) = parent.get(&self.path);
            entities.insert(get_key_alias(key), val);
        }
        if let Some(datatype) = &self.datatype {
            entities.insert("datatype", &self.path[datatype.clone()]);
        }
        if let Some(suffix) = &self.suffix {
            entities.insert("suffix", &self.path[suffix.clone()]);
        }
        if let Some(extension) = &self.extension {
            entities.insert("extension", &self.path[extension.clone()]);
        }
        entities
    }

    pub fn get_root(&self) -> &str {
        &self.path[..self.root]
    }

    pub fn get_head(&self) -> &str {
        &self.path[..self.head]
    }

    pub fn push_uncertain_datatype(&mut self, datatype: UnknownDatatypeTypes) {
        if let Some(dt) = self.uncertain_datatypes.as_mut() {
            dt.push(datatype)
        } else {
            self.uncertain_datatypes = Some(vec![datatype])
        }
    }

    pub fn extend_parts(&mut self, part: Vec<Range<usize>>) {
        if let Some(parts) = self.parts.as_mut() {
            parts.extend(part)
        } else {
            self.parts = Some(part)
        }
    }

    pub fn push_part(&mut self, part: Range<usize>) {
        if let Some(parts) = self.parts.as_mut() {
            parts.push(part)
        } else {
            self.parts = Some(vec![part])
        }
    }

    /// Modifies the provided range to cover just the suffix. Returns range for extension,
    /// if found
    pub fn extract_extension(&self, range: &mut Range<usize>) -> Option<Range<usize>> {
        self.path[range.clone()].find('.').and_then(|i| {
            let end = range.end;
            range.end = range.start + i;
            Some(range.start + i..end)
        })
    }

    // pub cleanup_uncertain_datatypes()
}

impl std::ops::Index<Range<usize>> for BidsPath {
    type Output = str;
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.path[index]
    }
}

impl std::ops::Index<&Range<usize>> for BidsPath {
    type Output = str;
    fn index(&self, index: &Range<usize>) -> &Self::Output {
        &self.path[index.clone()]
    }
}
