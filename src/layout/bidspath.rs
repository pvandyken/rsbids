use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    ops::Range,
    path::Path,
    hash::Hash,
};

use itertools::chain;
use serde::{Deserialize, Serialize};

use crate::{errors::MetadataReadErr, standards::get_key_alias};

use super::{builders::primitives::KeyVal, utfpath::UtfPath};

pub type MetadataReadResult = Result<HashMap<String, String>, MetadataReadErr>;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnknownDatatypeTypes {
    Linked(String, UnknownDatatype),
    Unlinked(UnknownDatatype),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidsPath {
    pub path: UtfPath,
    pub entities: Vec<KeyVal>,
    pub parts: Option<Vec<Range<usize>>>,
    pub suffix: Option<Range<usize>>,
    pub extension: Option<Range<usize>>,
    pub datatype: Option<Range<usize>>,
    pub parents: Vec<KeyVal>,
    pub head: usize,
    pub root: usize,
    pub depth: usize,
    pub uncertain_parents: Option<Vec<KeyVal>>,
    pub uncertain_datatypes: Option<Vec<UnknownDatatypeTypes>>,
}

impl BidsPath {
    pub fn new(path: UtfPath, root: usize, depth: usize) -> BidsPath {
        BidsPath {
            path,
            depth,
            head: 0,
            entities: vec![],
            parts: None,
            suffix: None,
            extension: None,
            datatype: None,
            parents: vec![],
            root,
            uncertain_parents: None,
            uncertain_datatypes: None,
        }
    }

    /// Return a borrowed Pathbuf view of path
    pub fn as_path(&self) -> &Path {
        self.path.as_path()
    }
    /// Return a str view of the path.
    ///
    /// Panics if path cannot be converted because it is not valid unicode. This function
    /// assumes non-unicode paths have already been appropriately handled during BidsPath
    /// construction
    pub fn as_str(&self) -> &str {
        &self.path.as_str()
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
        let mut uncertain_parents = None;
        std::mem::swap(&mut self.uncertain_parents, &mut uncertain_parents);
        for parent in uncertain_parents.as_mut()?.drain(..) {
            let key = parent.get_key(self.as_str());
            if parents.contains(key) {
                self.parents.push(parent)
            }
        }
        self.uncertain_parents = None;
        Some(())
    }

    pub fn get_full_entities(&self) -> HashMap<&str, &str> {
        let mut entities = HashMap::new();
        for parent in chain![&self.parents, &self.entities] {
            let (key, val) = parent.get(&self.as_str());
            entities.insert(get_key_alias(key), val);
        }
        if let Some(datatype) = &self.datatype {
            entities.insert("datatype", &self.as_str()[datatype.clone()]);
        }
        if let Some(suffix) = &self.suffix {
            entities.insert("suffix", &self.as_str()[suffix.clone()]);
        }
        if let Some(extension) = &self.extension {
            entities.insert("extension", &self.as_str()[extension.clone()]);
        }
        entities
    }

    pub fn get_entities(&self) -> HashMap<&str, &str> {
        let mut entities = HashMap::new();
        for parent in chain![&self.parents, &self.entities] {
            let (key, val) = parent.get(&self.as_str());
            entities.insert(key, val);
        }
        if let Some(datatype) = &self.datatype {
            entities.insert("datatype", &self.as_str()[datatype.clone()]);
        }
        if let Some(suffix) = &self.suffix {
            entities.insert("suffix", &self.as_str()[suffix.clone()]);
        }
        if let Some(extension) = &self.extension {
            entities.insert("extension", &self.as_str()[extension.clone()]);
        }
        entities
    }

    pub fn get_uncertain_entities(&self) -> Option<HashMap<&str, &str>> {
        if let Some(uncertain_parents) = self.uncertain_parents.as_ref() {
            let mut entities = HashMap::new();
            for parent in uncertain_parents {
                let (key, val) = parent.get(&self.as_str());
                entities.insert(key, val);
            }
            Some(entities)
        } else {
            None
        }
    }

    pub fn get_root(&self) -> &str {
        &self.as_str()[..self.root]
    }

    pub fn get_head(&self) -> &str {
        &self.as_str()[..self.head]
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
        self.as_str()[range.clone()].find('.').and_then(|i| {
            let end = range.end;
            range.end = range.start + i;
            Some(range.start + i..end)
        })
    }

    pub fn read_as_metadata(&self) -> Result<HashMap<String, serde_json::Value>, MetadataReadErr> {
        let mut file = File::open(&self.as_path())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let parsed: HashMap<String, serde_json::Value> = serde_json::from_str(&contents)
            .map_err(|err| MetadataReadErr::Json(self.as_str().to_string(), err))?;
        Ok(parsed)
    }

    /// Create a fresh BidsPath without any entity annotations (just depth and root)
    pub fn clear(self) -> Self {
        Self::new(self.path.clone(), self.root, self.depth)
    }
}

impl std::ops::Index<Range<usize>> for BidsPath {
    type Output = str;
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.as_str()[index]
    }
}

impl std::ops::Index<&Range<usize>> for BidsPath {
    type Output = str;
    fn index(&self, index: &Range<usize>) -> &Self::Output {
        &self.as_str()[index.clone()]
    }
}

impl Hash for BidsPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_path().hash(state);
        self.root.hash(state);
    }
}

impl PartialEq for BidsPath {
    fn eq(&self, other: &Self) -> bool {
        self.as_path() == other.as_path() && self.root == other.root
    }
}

impl Eq for BidsPath {}