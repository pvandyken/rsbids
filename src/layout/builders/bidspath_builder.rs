use std::{
    ops::Range,
    path::{Component, Path},
};

use crate::layout::bidspath::{BidsPath, UnknownDatatypeTypes};

use super::primitives::{ComponentType, Elements, KeyVal, PrePrimitive, Primitive};

pub struct BidsPathComponents {
    pub components: Vec<ComponentType>,
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

    pub fn no_parse(self) -> BidsPath {
        BidsPath::new(self.path, self.root)
    }
}
