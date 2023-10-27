use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::{
    bidspath::{BidsPath, BidsPathPart, Name, UnknownDatatype, UnknownDatatypeTypes, BidsPathComponents},
    primitives::ComponentType,
    standards::{BIDS_DATATYPES, BIDS_ENTITIES},
};

type EntityTable = HashMap<String, HashMap<String, Vec<usize>>>;

trait EntityTableExt {
    fn insert_entity(&mut self, i: usize, entity: &str, value: &str);
}

impl EntityTableExt for EntityTable {
    fn insert_entity(&mut self, i: usize, entity: &str, value: &str) {
        if let Some(val_map) = self.get_mut(entity) {
            if let Some(vec) = val_map.get_mut(value) {
                vec.push(i);
            } else {
                val_map.insert(value.to_string(), vec![i]);
            }
        } else {
            let mut val_map = HashMap::new();
            val_map.insert(value.to_string(), vec![i]);
            self.insert(entity.to_string(), val_map);
        }
    }
}

pub fn check_datatype(datatype: &str) -> bool {
    BIDS_DATATYPES.contains(datatype)
}

#[derive(Debug, Default)]
pub struct Dataset {
    paths: Vec<BidsPath>,
    entities: EntityTable,
    unknown_entities: EntityTable,
    unknown_datatypes: HashSet<usize>,
}

impl Dataset {
    pub fn num_paths(&self) -> usize {
        self.paths.len()
    }

    pub fn get_path(&self, index: usize) -> Option<&str> {
        Some(&self.paths.get(index)?.path)
    }
}


impl Dataset {
    fn add_entity(&mut self, i: usize, entity: &str, value: &str) {
        if self.check_entity(entity) {
            self.entities.insert_entity(i, entity, value)
        } else {
            self.unknown_entities.insert_entity(i, entity, value)
        }
    }

    fn confirm_entity(&mut self, entity: &str) {
        if let Some((entity, value)) = self.unknown_entities.remove_entry(entity) {
            self.entities.insert(entity, value);
        }
    }
    fn add_and_confirm_entity(&mut self, i: usize, entity: &str, value: &str) {
        self.confirm_entity(entity);
        self.entities.insert_entity(i, entity, value)
    }
    fn check_entity(&self, entity: &str) -> bool {
        self.entities.contains_key(entity) || BIDS_ENTITIES.contains(entity)
    }


    fn add_uncertain_datatype(&mut self, i: usize) {
        self.unknown_datatypes.insert(i);
    }

    pub fn label_component_type<'b>(
        &self,
        previous: &BidsPathPart,
        comp: ComponentType,
        template: &str,
        next_is_twotype: bool,
    ) -> BidsPathPart {
        match comp {
            ComponentType::TwoType(elems) => BidsPathPart::Name(Name::from_twotype(elems)),
            ComponentType::OneType(keyval) => match previous {
                BidsPathPart::Head => {
                    if self.check_entity(keyval.get_key(template)) {
                        BidsPathPart::Parent(keyval)
                    } else {
                        BidsPathPart::UncertainParent(keyval)
                    }
                }
                BidsPathPart::Datatype(..) | BidsPathPart::Name(..) => {
                    BidsPathPart::Name(Name::from_onetype(keyval))
                }
                BidsPathPart::Parent(..) => BidsPathPart::Parent(keyval),
                BidsPathPart::UncertainParent(..) | BidsPathPart::UncertainDatatype(..) => {
                    BidsPathPart::UncertainParent(keyval)
                }
            },
            ComponentType::ZeroType(comp) => match previous {
                BidsPathPart::Head => {
                    if next_is_twotype || check_datatype(&template[comp.clone()]) {
                        BidsPathPart::Datatype(comp)
                    } else {
                        BidsPathPart::Head
                    }
                }
                BidsPathPart::Datatype(..) | BidsPathPart::Name(..) => {
                    BidsPathPart::Name(Name::from_zerotype(comp))
                }
                BidsPathPart::Parent(..) => BidsPathPart::Datatype(comp),
                BidsPathPart::UncertainDatatype(..) => {
                    let is_valid = next_is_twotype || check_datatype(&template[comp.clone()]);
                    BidsPathPart::UncertainDatatype(UnknownDatatypeTypes::Unlinked(
                        UnknownDatatype::new(comp, is_valid),
                    ))
                }
                BidsPathPart::UncertainParent(keyval) => {
                    let is_valid = next_is_twotype || check_datatype(&template[comp.clone()]);
                    BidsPathPart::UncertainDatatype(UnknownDatatypeTypes::Linked(
                        keyval.get_key(template).to_string(),
                        UnknownDatatype::new(comp, is_valid),
                    ))
                }
            },
        }
    }

    pub fn add_path(&mut self, path: String) {
        let next_i = self.paths.len();
        let bidsparts = BidsPathComponents::to_bidsparts(&path, self);
        let mut bidspath = BidsPath::new(path);

        self.collect_elements(next_i, &mut bidspath, bidsparts);
        self.paths.push(bidspath);
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

    pub fn cleanup(&mut self) {
        self.unknown_entities.clear();
        self.unknown_entities.shrink_to_fit();
        let unknown_datatypes = self.unknown_datatypes.drain().collect_vec();
        self.unknown_datatypes.shrink_to_fit();
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
    }

    fn collect_elements(&mut self, path_i: usize, path: &mut BidsPath, parts: Vec<BidsPathPart>) {
        let mut named_entities = HashSet::new();
        for (i, part) in parts.into_iter().rev().enumerate() {
            match part {
                BidsPathPart::Head => (),
                BidsPathPart::Parent(keyval) => {
                    let (key, value) = keyval.get(&path.path);
                    if !named_entities.contains(key) {
                        self.add_and_confirm_entity(path_i, key, value);
                        path.parents.push(keyval)
                    }
                }
                BidsPathPart::UncertainParent(keyval) => {
                    let (key, value) = keyval.get(&path.path);
                    self.add_entity(path_i, key, value);
                    path.add_uncertain_parent(keyval)
                }
                BidsPathPart::Datatype(comp) => {
                    self.add_entity(path_i, "datatype", &path[&comp]);
                    path.datatype = Some(comp)
                }
                BidsPathPart::Name(mut name) => {
                    if let Some(parts) = name.parts {
                        path.extend_parts(parts)
                    }
                    if i == 0 {
                        if let Some(mut suffix) = name.suffix {
                            if let Some(extension) = path.extract_extension(&mut suffix) {
                                self.add_entity(path_i, "extension", &path[&extension]);
                                path.extension = Some(extension);
                            }
                            self.add_entity(path_i, "suffix", &path[&suffix]);
                            path.suffix = Some(suffix)
                        } else if let Some(keyval) = name.entities.as_mut().and_then(|kv| kv.pop())
                        {
                            if let Some(extension) = path.extract_extension(&mut keyval.val_range())
                            {
                                self.add_entity(path_i, "extension", &path[&extension]);
                                path.extension = Some(extension);
                            }
                            let (key, value) = keyval.get(&path.path);
                            self.add_and_confirm_entity(path_i, key, value);
                            named_entities.insert(keyval.get_key(&path.path).to_string());
                            path.entities.push(keyval);
                        }
                    } else if let Some(suffix) = name.suffix {
                        path.push_part(suffix)
                    }
                    if let Some(entities) = name.entities {
                        for entity in entities {
                            let (key, value) = entity.get(&path.path);
                            self.add_and_confirm_entity(path_i, key, value);
                            named_entities.insert(entity.get_key(&path.path).to_string());
                            path.entities.push(entity);
                        }
                    }
                }
                BidsPathPart::UncertainDatatype(datatype) => {
                    self.add_uncertain_datatype(path_i);
                    path.push_uncertain_datatype(datatype)
                }
            }
        }
    }
}
