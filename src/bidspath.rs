use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    path::{Component, Path},
};

use itertools::chain;

use crate::{
    dataset::{check_datatype, entity_table::EntityTable},
    primitives::{ComponentType, Elements, KeyVal, PrePrimitive, Primitive},
    standards::{get_key_alias, BIDS_ENTITIES},
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

#[derive(Debug)]
pub enum BidsPathPart {
    Head(usize),
    Parent(KeyVal),
    Datatype(Range<usize>),
    Name(Name),
    UncertainParent(KeyVal),
    UncertainDatatype(UnknownDatatypeTypes),
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

fn get_next_elem(path: &str) -> PrePrimitive {
    match path.rfind(['_', '-']) {
        Some(i) => match path.as_bytes()[i] {
            b'_' => PrePrimitive::KeyLike(i),
            b'-' => PrePrimitive::ValueLike(i),
            _ => panic!("Unexpected match found!"),
        },
        None => PrePrimitive::Prefix,
    }
}

fn process_keylike(start: usize, end: usize, last_elem: Option<&Primitive>) -> Primitive {
    match last_elem {
        Some(last) => match last {
            Primitive::Suffix(..) => Primitive::Suffix(start, end),
            Primitive::Value(..) => Primitive::Key(start, end),
            Primitive::Key(..) | Primitive::Part(..) => Primitive::Part(start, end),
        },
        None => Primitive::Suffix(start, end),
    }
}
pub fn parse_path_segment<'a>(component: Range<usize>, template: &str) -> Vec<Primitive> {
    fn recurse(path: &str, start_i: usize, offset: usize, elems: &mut Vec<Primitive>) {
        let mut next_i = 0;
        let mut finish = false;
        let next_elem = match get_next_elem(&path[..start_i]) {
            PrePrimitive::KeyLike(i) => {
                next_i = i;
                process_keylike(i + offset + 1, start_i + offset, elems.last())
            }
            PrePrimitive::ValueLike(i) => {
                next_i = i;
                Primitive::Value(i + offset + 1, start_i + offset)
            }
            PrePrimitive::Prefix => {
                finish = true;
                process_keylike(offset, start_i + offset, elems.last())
            }
        };
        elems.push(next_elem);
        if finish {
            return;
        } else {
            recurse(path, next_i, offset, elems)
        }
    }
    let mut primitives = Vec::new();
    let start = component.start;
    let len = component.len();
    recurse(&template[component], len, start, &mut primitives);
    primitives
}

pub fn classify_component<'a>(mut elements: Vec<Elements>) -> ComponentType {
    if elements.len() > 1 {
        return ComponentType::TwoType(elements);
    }
    match elements.pop().expect("Should have at least one element") {
        Elements::Suffix(suffix) => ComponentType::ZeroType(suffix),
        Elements::KeyVal(keyval) => ComponentType::OneType(keyval),
        Elements::Part(..) => panic!("Should not have a part as the only element in a component"),
    }
}

pub fn get_components(path: &str) -> Vec<Range<usize>> {
    // let path = Path::new(&path);
    let mut components = Vec::new();
    let mut curr_i = 0;
    for component in Path::new(&path).components() {
        let incr = match component {
            Component::Normal(comp) => {
                let incr = comp.len();
                components.push(curr_i..curr_i + incr);
                incr + 1
            }
            Component::Prefix(prefix) => prefix.as_os_str().len() + 1,
            Component::RootDir => 1,
            Component::CurDir => 2,
            Component::ParentDir => 3,
        };
        curr_i += incr;
    }
    components
}

fn consume_values<'a>(data: &mut Vec<Primitive>, keystart: usize, keyend: usize) -> Elements {
    let mut end = keyend + 1;
    while let Some(last) = data.pop() {
        match last {
            Primitive::Value(_s, e) => end = e,
            _ => {
                panic!("Should not have encountered a non-value within the consume_values loop")
            }
        }
        if data
            .last()
            .is_some_and(|next| -> bool { !matches!(next, Primitive::Value(..)) })
        {
            break;
        }
    }
    Elements::KeyVal(KeyVal::new(keystart..end, keyend))
}

pub fn group_primitives(mut data: Vec<Primitive>) -> Vec<Elements> {
    let mut elems = Vec::new();
    while let Some(last) = data.pop() {
        let mut finish = false;
        let grouped = match last {
            Primitive::Key(start, end) => consume_values(&mut data, start, end),
            Primitive::Suffix(start, _end) => {
                finish = true;
                Elements::Suffix(start.._end)
            }
            Primitive::Part(start, end) => Elements::Part(start..end),
            Primitive::Value(..) => {
                panic!("Values should have been consumed within the key block")
            }
        };
        elems.push(grouped);
        if finish {
            break;
        }
    }
    elems
}

pub struct BidsPathComponents {
    pub components: Vec<ComponentType>,
}
pub fn to_bidsparts_patch(path: &str) -> Vec<ComponentType> {
    let components = get_components(&path);
    let mut comps = Vec::new();
    for component in components {
        let elements = parse_path_segment(component, &path);
        let elements = group_primitives(elements);
        comps.push(classify_component(elements));
    }
    comps
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

#[derive(Debug, Default)]
pub struct Name {
    pub entities: Option<Vec<KeyVal>>,
    pub parts: Option<Vec<Range<usize>>>,
    pub suffix: Option<Range<usize>>,
}

impl Name {
    pub fn from_twotype(elems: Vec<Elements>) -> Name {
        let mut name = Name::default();
        let mut entities = Vec::new();
        let mut parts = Vec::new();
        for elem in elems {
            match elem {
                Elements::KeyVal(keyval) => entities.push(keyval),
                Elements::Part(part) => parts.push(part),
                Elements::Suffix(s) => name.suffix = Some(s),
            }
        }
        if !entities.is_empty() {
            name.entities = Some(entities)
        }
        if !parts.is_empty() {
            name.parts = Some(parts)
        }
        name
    }

    pub fn from_onetype(keyval: KeyVal) -> Name {
        Name {
            entities: Some(Vec::from([keyval])),
            parts: None,
            suffix: None,
        }
    }

    pub fn from_zerotype(comp: Range<usize>) -> Name {
        let mut name = Name::default();
        name.suffix = Some(comp);
        name
    }
}

#[derive(Debug)]
pub struct BidsPathBuilder {
    pub path: String,
    pub components: Vec<ComponentType>,
    pub root: usize,
}

impl BidsPathBuilder {
    pub fn new(path: String, root: usize) -> Self {
        let components = get_components(&path);
        let mut comps = Vec::new();
        for component in components {
            let elements = parse_path_segment(component, &path);
            let elements = group_primitives(elements);
            comps.push(classify_component(elements));
        }
        Self {
            path,
            components: comps,
            root,
        }
    }

    pub fn with_seperate_labels(
        self,
        known_entities: &EntityTable,
    ) -> (BidsPath, Vec<BidsPathPart>) {
        let is_twotype: Vec<bool> = self
            .components
            .iter()
            .map(|comp| matches!(comp, ComponentType::TwoType(..)))
            .collect();
        let mut labelled = Vec::new();

        let len = self.components.len();
        for (i, comp) in self.components.into_iter().enumerate() {
            if i + 1 == len {
                labelled.push(BidsPathPart::Name(match comp {
                    ComponentType::OneType(keyval) => Name::from_onetype(keyval),
                    ComponentType::TwoType(elems) => Name::from_twotype(elems),
                    ComponentType::ZeroType(comp) => Name::from_zerotype(comp),
                }));
                break;
            }
            let next_is_twotype = is_twotype[i + 1];
            labelled.push(Self::label_component_type(
                labelled.last().unwrap_or(&BidsPathPart::Head(0)),
                comp,
                &self.path,
                next_is_twotype,
                known_entities,
            ));
        }
        (BidsPath::new(self.path, self.root), labelled)
    }

    pub fn via_spec(self) -> Result<BidsPath, Self> {
        let mut bidspath = BidsPath::new(self.path, self.root);
        let mut lastmatch = BidsPathPart::Head(0);
        let known_entities: EntityTable = HashMap::new();
        let len = self.components.len();
        for (i, comp) in self.components.into_iter().enumerate() {
            match comp {
                ComponentType::ZeroType(range) => match lastmatch {
                    BidsPathPart::Head(..) => {
                        if check_datatype(&bidspath.path[range.clone()]) {
                            bidspath.datatype = Some(range.clone());
                            lastmatch = BidsPathPart::Datatype(range);
                        } else {
                            bidspath.head = range.end;
                            lastmatch = BidsPathPart::Head(range.end)
                        }
                    }
                    BidsPathPart::Parent(..) => {
                        if check_datatype(&bidspath.path[range.clone()]) {
                            bidspath.datatype = Some(range.clone());
                            lastmatch = BidsPathPart::Datatype(range);
                        } else if i + 1 == len {
                            // Last component can't be zero-type
                            return Err(Self::new(bidspath.path, bidspath.root));
                        } else {
                            bidspath.push_part(range.clone());
                            lastmatch = BidsPathPart::Name(Name::from_zerotype(range));
                        }
                    }
                    BidsPathPart::Datatype(..) | BidsPathPart::Name(..) => {
                        if i + 1 == len {
                            // Last component can't be zero-type
                            return Err(Self::new(bidspath.path, bidspath.root));
                        } else {
                            bidspath.push_part(range.clone());
                            lastmatch = BidsPathPart::Name(Name::from_zerotype(range));
                        }
                    }
                    _ => panic!("Uncertain types should not be possible here")
                },
                ComponentType::OneType(keyval) => match lastmatch {
                    BidsPathPart::Head(..) => {
                        if Self::check_entity(keyval.get_key(&bidspath.path), &known_entities) {
                            bidspath.parents.push(keyval.clone());
                            lastmatch = BidsPathPart::Parent(keyval);
                        } else {
                            bidspath.head = keyval.slice.end;
                            lastmatch = BidsPathPart::Head(keyval.slice.end)
                        }
                    }
                    BidsPathPart::Parent(..) => {
                        if Self::check_entity(keyval.get_key(&bidspath.path), &known_entities) {
                            bidspath.parents.push(keyval.clone());
                            lastmatch = BidsPathPart::Parent(keyval);
                        } else if i + 1 == len {
                            // Last component can't be one-type
                            return Err(Self::new(bidspath.path, bidspath.root));
                        } else {
                            bidspath.push_part(keyval.slice.clone());
                            lastmatch = BidsPathPart::Name(Name::from_zerotype(keyval.slice));
                        }
                    }
                    BidsPathPart::Datatype(..) | BidsPathPart::Name(..) => {
                        if Self::check_entity(keyval.get_key(&bidspath.path), &known_entities) {
                            bidspath.parents.push(keyval.clone());
                            lastmatch = BidsPathPart::Parent(keyval);
                        } else if i + 1 == len {
                            // Last component can't be one-type
                            return Err(Self::new(bidspath.path, bidspath.root));
                        } else {
                            bidspath.push_part(keyval.slice.clone());
                            lastmatch = BidsPathPart::Name(Name::from_zerotype(keyval.slice));
                        }
                    }
                    _ => panic!("Uncertain types should not be possible here")
                },
                ComponentType::TwoType(elems) => {
                    lastmatch = BidsPathPart::Name(Name::from_zerotype(0..0));
                    for (j, elem) in elems.into_iter().rev().enumerate() {
                        if j == 0 && i + 1 == len {
                            match elem {
                                Elements::Suffix(range) => {
                                    bidspath.suffix = Some(range);
                                }
                                _ => {
                                    // Very last element must be suffix
                                    return Err(Self::new(bidspath.path, bidspath.root));
                                }
                            }
                        } else {
                            match elem {
                                Elements::KeyVal(keyval) => {
                                    if Self::check_entity(
                                        keyval.get_key(&bidspath.path),
                                        &known_entities,
                                    ) {
                                        bidspath.parents.push(keyval.clone());
                                    } else {
                                        bidspath.push_part(keyval.slice.clone());
                                    }
                                }
                                Elements::Part(range) | Elements::Suffix(range) => {
                                    bidspath.push_part(range)
                                }
                            }
                        }
                    }

                }
            }
        }
        Ok(bidspath)
    }

    fn check_entity(entity: &str, known_entities: &EntityTable) -> bool {
        known_entities.contains_key(entity) || BIDS_ENTITIES.contains_left(entity)
    }

    fn label_component_type<'b>(
        previous: &BidsPathPart,
        comp: ComponentType,
        template: &str,
        next_is_twotype: bool,
        known_entities: &EntityTable,
    ) -> BidsPathPart {
        match comp {
            ComponentType::TwoType(elems) => BidsPathPart::Name(Name::from_twotype(elems)),
            ComponentType::OneType(keyval) => match previous {
                BidsPathPart::Head(..) => {
                    if Self::check_entity(keyval.get_key(template), known_entities) {
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
                BidsPathPart::Head(..) => {
                    if next_is_twotype || check_datatype(&template[comp.clone()]) {
                        BidsPathPart::Datatype(comp)
                    } else {
                        BidsPathPart::Head(comp.end)
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
}
